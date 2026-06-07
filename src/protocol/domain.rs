#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};
#[cfg(feature = "std")]
use std::string::{String, ToString};

use crate::core::{BitBoard, GameStatus as CoreGameStatus, GuessBoardState, BOARD_SIZE};

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Board {/* grid, ships, hits/misses */}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Ship {
    pub name: String,
    pub sunk: bool,
    pub position: Option<(u8, u8, crate::core::ship::Orientation)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum GuessResult {
    Hit,
    Miss,
    Sink { ship: String, footprint: BB },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPayload {
    /// Complete game state for full synchronization
    pub game_state: crate::core::game::GameState,
    /// Which enemy ships are still afloat (by index into SHIPS array)
    pub enemy_ships_remaining: [bool; crate::core::config::NUM_SHIPS],
}

/// Player identity inside private remote synchronization payloads.
///
/// Values are relative to the sender: `Local` is the sending app, and `Remote`
/// is the connected peer. This keeps private sync payloads free of board state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum RemotePlayer {
    Local,
    Remote,
}

/// Hidden-information-safe remote synchronization payload.
///
/// This payload intentionally carries only public shot history and match
/// metadata. It must not include `BoardState`, ship placements, or complete
/// `GameState` values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct RemoteSyncPayload {
    pub turn_number: u32,
    pub active_player: RemotePlayer,
    pub next_seq: u64,
    pub last_received_seq: Option<u64>,
    pub public_shots: GuessBoardState,
    pub enemy_ships_remaining: [bool; crate::core::config::NUM_SHIPS],
    pub enemy_remaining: usize,
    pub status: GameStatus,
}

impl From<crate::core::common::GuessResult> for GuessResult {
    fn from(res: crate::core::common::GuessResult) -> Self {
        match res {
            crate::core::common::GuessResult::Hit => GuessResult::Hit,
            crate::core::common::GuessResult::Miss => GuessResult::Miss,
            crate::core::common::GuessResult::Sink(name) => GuessResult::Sink {
                ship: name.to_string(),
                footprint: BB::new(),
            },
        }
    }
}

impl From<CoreGameStatus> for GameStatus {
    fn from(status: CoreGameStatus) -> Self {
        match status {
            CoreGameStatus::InProgress => GameStatus::InProgress,
            CoreGameStatus::Won => GameStatus::Won,
            CoreGameStatus::Lost => GameStatus::Lost,
        }
    }
}

impl From<crate::core::ship::Ship<u128, { BOARD_SIZE as usize }>> for Ship {
    fn from(state: crate::core::ship::Ship<u128, { BOARD_SIZE as usize }>) -> Self {
        Ship {
            name: state.name.to_string(),
            sunk: state.sunk,
            position: state.position.map(|(r, c, o)| (r as u8, c as u8, o)),
        }
    }
}
