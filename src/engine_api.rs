use crate::domain::*;
#[async_trait::async_trait]
pub trait GameApi: Send + Sync {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult>;
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship>;
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()>;
    fn status(&self) -> GameStatus;
}
