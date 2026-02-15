#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};
#[cfg(feature = "std")]
use std::string::{String, ToString};

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Board {/* grid, ships, hits/misses */}
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Ship {
    pub name: String,
    pub sunk: bool,
    pub position: Option<(u8, u8, crate::core::ship::Orientation)>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum GuessResult {
    Hit,
    Miss,
    Sink(String),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SyncPayload {
    /// Complete game state for full synchronization
    pub game_state: crate::core::game::GameState,
    /// Which enemy ships are still afloat (by index into SHIPS array)
    pub enemy_ships_remaining: [bool; crate::core::config::NUM_SHIPS as usize],
}

impl From<crate::core::common::GuessResult> for GuessResult {
    fn from(res: crate::core::common::GuessResult) -> Self {
        match res {
            crate::core::common::GuessResult::Hit => GuessResult::Hit,
            crate::core::common::GuessResult::Miss => GuessResult::Miss,
            crate::core::common::GuessResult::Sink(name) => GuessResult::Sink(name.to_string()),
        }
    }
}

impl From<crate::core::ship::ShipState> for Ship {
    fn from(state: crate::core::ship::ShipState) -> Self {
        Ship {
            name: state.name.to_string(),
            sunk: state.sunk,
            position: state.position.map(|(r, c, o)| (r as u8, c as u8, o)),
        }
    }
}
