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
    pub position: Option<(u8, u8, crate::ship::Orientation)>,
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
    pub game_state: crate::game::GameState,
    /// Which enemy ships are still afloat (by index into SHIPS array)
    pub enemy_ships_remaining: [bool; crate::config::NUM_SHIPS as usize],
}

impl From<crate::common::GuessResult> for GuessResult {
    fn from(res: crate::common::GuessResult) -> Self {
        match res {
            crate::common::GuessResult::Hit => GuessResult::Hit,
            crate::common::GuessResult::Miss => GuessResult::Miss,
            crate::common::GuessResult::Sink(name) => GuessResult::Sink(name.to_string()),
        }
    }
}

impl From<crate::ship::ShipState> for Ship {
    fn from(state: crate::ship::ShipState) -> Self {
        Ship {
            name: state.name.to_string(),
            sunk: state.sunk,
            position: state.position.map(|(r, c, o)| (r as u8, c as u8, o)),
        }
    }
}
