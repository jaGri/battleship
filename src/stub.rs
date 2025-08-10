#![cfg(feature = "std")]

use crate::domain::{GameStatus, GuessResult, Ship, SyncPayload};
use crate::{
    protocol::{GameApi, Message, PROTOCOL_VERSION},
    transport::Transport,
};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;

pub struct Stub<T: Transport> {
    transport: Mutex<T>,
    seq: AtomicU64,
}

impl<T: Transport> Stub<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Mutex::new(transport),
            seq: AtomicU64::new(0),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }
}
#[async_trait::async_trait]
impl<T: Transport> GameApi for Stub<T> {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult> {
        let mut transport = self.transport.lock().await;
        let seq = self.next_seq();
        transport
            .send(Message::Guess {
                version: PROTOCOL_VERSION,
                seq,
                x,
                y,
            })
            .await?;
        match transport.recv().await? {
            Message::StatusResp {
                seq: resp_seq, res, ..
            } if resp_seq == seq => Ok(res),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship> {
        let mut transport = self.transport.lock().await;
        let seq = self.next_seq();
        transport
            .send(Message::ShipStatusReq {
                version: PROTOCOL_VERSION,
                seq,
                id: ship_id,
            })
            .await?;
        match transport.recv().await? {
            Message::ShipStatusResp {
                seq: resp_seq,
                ship,
                ..
            } if resp_seq == seq => Ok(ship),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()> {
        let mut transport = self.transport.lock().await;
        let seq = self.next_seq();
        transport
            .send(Message::Sync {
                version: PROTOCOL_VERSION,
                seq,
                payload,
            })
            .await?;
        match transport.recv().await? {
            Message::Ack { seq: resp_seq, .. } if resp_seq == seq => Ok(()),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    fn status(&self) -> GameStatus {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut transport = self.transport.lock().await;
                let seq = self.next_seq();
                transport
                    .send(Message::GameStatusReq {
                        version: PROTOCOL_VERSION,
                        seq,
                    })
                    .await
                    .unwrap();
                match transport.recv().await.unwrap() {
                    Message::GameStatusResp {
                        seq: resp_seq,
                        status,
                        ..
                    } if resp_seq == seq => status,
                    _ => panic!("Unexpected message"),
                }
            })
        })
    }
}
