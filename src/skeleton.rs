#![cfg(feature = "std")]

use crate::{protocol::GameApi, protocol::Message, transport::Transport};
use anyhow::anyhow;

pub struct Skeleton<E: GameApi, T: Transport> {
    engine: E,
    transport: T,
}

impl<E: GameApi, T: Transport> Skeleton<E, T> {
    pub fn new(engine: E, transport: T) -> Self {
        Self { engine, transport }
    }
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Perform handshake
        match self.transport.recv().await? {
            Message::Hello { version, session } => {
                self.transport.send(Message::Hello { version, session }).await?;
            }
            _ => return Err(anyhow!("Expected handshake")),
        }

        while let Ok(msg) = self.transport.recv().await {
            let reply = match msg {
                Message::Guess { x, y } => {
                    let res = self.engine.make_guess(x, y).await?;
                    Message::StatusResp(res)
                }
                Message::StatusReq | Message::GameStatusReq => {
                    let status = self.engine.status();
                    Message::GameStatusResp(status)
                }
                Message::ShipStatusReq { id } => {
                    let ship = self.engine.get_ship_status(id).await?;
                    Message::ShipStatusResp(ship)
                }
                Message::Sync(payload) => { self.engine.sync_state(payload).await?; Message::Ack },
                Message::Resync { state } => { self.engine.resync(state).await?; Message::Ack },
                _ => Message::Ack,
            };
            self.transport.send(reply).await?;
        }
        Ok(())
    }
}