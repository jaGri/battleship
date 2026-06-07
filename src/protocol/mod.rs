//! Network protocol definitions and RPC framework
//!
//! This module defines the protocol for communicating between game instances:
//! - WireMessage enum: All protocol messages (Handshake, Guess, StatusResp, etc.)
//! - Domain types: Serializable versions of game types

pub mod domain;

use domain::*;

/// Current protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

#[cfg(feature = "std")]
pub use async_trait;

/// Messages exchanged between the game engine and a remote client.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WireMessage {
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
