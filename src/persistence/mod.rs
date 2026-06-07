//! Adapter-neutral persistence types for app runners.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use crate::agent::{AiDifficulty, Coordinate};
use crate::app::SavedGame;

/// Persisted agent state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentSnapshot {
    pub difficulty: Option<AiDifficulty>,
    pub last_target: Option<Coordinate>,
    pub scripted_actions_remaining: u16,
}

/// Persisted UI state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct UiSnapshot {
    pub cursor: Option<Coordinate>,
    pub selected_menu_item: usize,
    pub messages: Vec<String>,
}

/// Adapter-specific state saved alongside a game.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum AdapterState {
    Agent(AgentSnapshot),
    Ui(UiSnapshot),
    None,
}

impl Default for AdapterState {
    fn default() -> Self {
        Self::None
    }
}

/// Simple active-save store used by app runners.
pub trait SaveStore {
    type Error;

    fn load_active(&mut self) -> Result<Option<SavedGame>, Self::Error>;
    fn save_active(&mut self, game: &SavedGame) -> Result<(), Self::Error>;
    fn clear_active(&mut self) -> Result<(), Self::Error>;
}

/// In-memory save store for tests and local simulations.
#[derive(Debug, Default, Clone)]
pub struct MemorySaveStore {
    active: Option<SavedGame>,
}

impl SaveStore for MemorySaveStore {
    type Error = core::convert::Infallible;

    fn load_active(&mut self) -> Result<Option<SavedGame>, Self::Error> {
        Ok(self.active)
    }

    fn save_active(&mut self, game: &SavedGame) -> Result<(), Self::Error> {
        self.active = Some(*game);
        Ok(())
    }

    fn clear_active(&mut self) -> Result<(), Self::Error> {
        self.active = None;
        Ok(())
    }
}
