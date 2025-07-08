use crate::common::GuessResult;

#[derive(Debug, Clone)]
pub struct SyncPayload;

#[cfg(feature = "std")]
use anyhow;

#[cfg(feature = "std")]
use async_trait::async_trait;
#[cfg(feature = "std")]
use alloc::boxed::Box;

/// Messages exchanged between the game engine and a remote client.
#[derive(Debug, Clone)]
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

#[cfg(feature = "std")]
#[async_trait]
pub trait GameApi {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult>;
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()>;
}
