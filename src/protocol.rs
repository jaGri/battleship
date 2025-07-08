use crate::domain::*;

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
