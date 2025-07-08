use crate::{protocol::GameApi, protocol::Message, transport::Transport};
pub struct Stub<T: Transport> { transport: T }
#[async_trait::async_trait]
impl<T: Transport> GameApi for Stub<T> {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult> {
        self.transport.send(Message::Guess { x, y }).await?;
        match self.transport.recv().await? {
            Message::StatusResp(res) => Ok(res),
            _ => Err(anyhow::anyhow!("Unexpected message")),
        }
    }
    /* implement other methods similarly */
    async fn get_ship_status(&self, _ship_id: usize) -> anyhow::Result<Ship> { unimplemented!() }
    async fn sync_state(&mut self, _payload: SyncPayload) -> anyhow::Result<()> { unimplemented!() }
    fn status(&self) -> GameStatus { unimplemented!() }
}