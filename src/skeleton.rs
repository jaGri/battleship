#![cfg(feature = "std")]

use crate::{
    protocol::{GameApi, Message, PROTOCOL_VERSION},
    transport::Transport,
};

pub struct Skeleton<E: GameApi, T: Transport> {
    engine: E,
    transport: T,
    next_seq: u64,
}

impl<E: GameApi, T: Transport> Skeleton<E, T> {
    pub fn new(engine: E, transport: T) -> Self {
        Self {
            engine,
            transport,
            next_seq: 0,
        }
    }
    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Ok(msg) = self.transport.recv().await {
            match msg {
                Message::Guess { version, seq, x, y } => {
                    if version != PROTOCOL_VERSION || seq != self.next_seq {
                        self.transport
                            .send(Message::Ack {
                                version: PROTOCOL_VERSION,
                                seq,
                            })
                            .await?;
                        continue;
                    }
                    self.next_seq += 1;
                    let res = self.engine.make_guess(x, y).await?;
                    self.transport
                        .send(Message::StatusResp {
                            version: PROTOCOL_VERSION,
                            seq,
                            res,
                        })
                        .await?;
                }
                Message::StatusReq { version, seq } | Message::GameStatusReq { version, seq } => {
                    if version != PROTOCOL_VERSION || seq != self.next_seq {
                        self.transport
                            .send(Message::Ack {
                                version: PROTOCOL_VERSION,
                                seq,
                            })
                            .await?;
                        continue;
                    }
                    self.next_seq += 1;
                    let status = self.engine.status();
                    self.transport
                        .send(Message::GameStatusResp {
                            version: PROTOCOL_VERSION,
                            seq,
                            status,
                        })
                        .await?;
                }
                Message::ShipStatusReq { version, seq, id } => {
                    if version != PROTOCOL_VERSION || seq != self.next_seq {
                        self.transport
                            .send(Message::Ack {
                                version: PROTOCOL_VERSION,
                                seq,
                            })
                            .await?;
                        continue;
                    }
                    self.next_seq += 1;
                    let ship = self.engine.get_ship_status(id).await?;
                    self.transport
                        .send(Message::ShipStatusResp {
                            version: PROTOCOL_VERSION,
                            seq,
                            ship,
                        })
                        .await?;
                }
                Message::Sync {
                    version,
                    seq,
                    payload,
                } => {
                    if version != PROTOCOL_VERSION || seq != self.next_seq {
                        self.transport
                            .send(Message::Ack {
                                version: PROTOCOL_VERSION,
                                seq,
                            })
                            .await?;
                        continue;
                    }
                    self.next_seq += 1;
                    self.engine.sync_state(payload).await?;
                    self.transport
                        .send(Message::Ack {
                            version: PROTOCOL_VERSION,
                            seq,
                        })
                        .await?;
                }
                Message::ShipStatusResp { .. }
                | Message::StatusResp { .. }
                | Message::GameStatusResp { .. }
                | Message::Ack { .. } => {
                    self.transport
                        .send(Message::Ack {
                            version: PROTOCOL_VERSION,
                            seq: self.next_seq,
                        })
                        .await?;
                }
            }
        }
        Ok(())
    }
}
