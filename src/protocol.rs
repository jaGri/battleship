use crate::domain::*;

/// Current protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

#[cfg(feature = "std")]
pub use async_trait;

/// Messages exchanged between the game engine and a remote client.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Message {
    /// Handshake message to establish connection and negotiate protocol version.
    Handshake { version: u8 },
    /// Acknowledgement of handshake with agreed version.
    HandshakeAck { version: u8 },
    /// Request to make a guess at the given coordinates.
    Guess { version: u8, seq: u64, x: u8, y: u8 },
    /// Request the current game status.
    StatusReq { version: u8, seq: u64 },
    /// Response carrying the result of a guess.
    StatusResp {
        version: u8,
        seq: u64,
        res: GuessResult,
    },
    /// Synchronise state between peers.
    Sync {
        version: u8,
        seq: u64,
        payload: SyncPayload,
    },
    /// Request the status of a particular ship by id.
    ShipStatusReq { version: u8, seq: u64, id: usize },
    /// Response containing the status of a ship.
    ShipStatusResp { version: u8, seq: u64, ship: Ship },
    /// Request the overall game status.
    GameStatusReq { version: u8, seq: u64 },
    /// Response containing the current game status.
    GameStatusResp {
        version: u8,
        seq: u64,
        status: GameStatus,
    },
    /// Generic acknowledgement.
    Ack { version: u8, seq: u64 },
    /// Heartbeat/keepalive to maintain connection.
    Heartbeat { version: u8 },
}

#[cfg_attr(feature = "std", async_trait::async_trait)]
pub trait GameApi: Send + Sync {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult>;
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship>;
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()>;
    fn status(&self) -> GameStatus;
}
