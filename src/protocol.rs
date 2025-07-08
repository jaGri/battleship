use crate::domain::*;

#[cfg(feature = "std")]
pub use async_trait;

/// Messages exchanged between the game engine and a remote client.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Message {
    /// Request to make a guess at the given coordinates.
    Guess { x: u8, y: u8 },
    /// Request the current game status.
    StatusReq,
    /// Response carrying the result of a guess.
    StatusResp(GuessResult),
    /// Synchronise state between peers.
    Sync(SyncPayload),
    /// Generic acknowledgement.
    Ack,
}

#[cfg_attr(feature = "std", async_trait::async_trait)]
pub trait GameApi: Send + Sync {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult>;
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship>;
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()>;
    fn status(&self) -> GameStatus;
}
