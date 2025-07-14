#![cfg(feature = "std")]

use crate::{protocol::GameApi, protocol::Message, transport::Transport};
use crate::domain::{GuessResult, Ship, SyncPayload, GameStatus};
use tokio::sync::Mutex;

pub struct Stub<T: Transport> {
    transport: Mutex<T>,
}

impl<T: Transport> Stub<T> {
    pub fn new(transport: T) -> Self {
        Self { transport: Mutex::new(transport) }
    }
}
#[async_trait::async_trait]
impl<T: Transport> GameApi for Stub<T> {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult> {
        let mut transport = self.transport.lock().await;
        transport.send(Message::Guess { x, y }).await?;
        match transport.recv().await? {
            Message::StatusResp(res) => Ok(res),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship> {
        let mut transport = self.transport.lock().await;
        transport.send(Message::ShipStatusReq { id: ship_id }).await?;
        match transport.recv().await? {
            Message::ShipStatusResp(ship) => Ok(ship),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()> {
        let mut transport = self.transport.lock().await;
        transport.send(Message::Sync(payload)).await?;
        match transport.recv().await? {
            Message::Ack => Ok(()),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    fn status(&self) -> GameStatus {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut transport = self.transport.lock().await;
                transport.send(Message::GameStatusReq).await.unwrap();
                match transport.recv().await.unwrap() {
                    Message::GameStatusResp(status) => status,
                    _ => panic!("Unexpected message"),
                }
            })
        })
    }
}