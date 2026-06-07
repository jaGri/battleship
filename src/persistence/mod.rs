//! Adapter-neutral persistence types for app runners.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
#[cfg(feature = "std")]
use std::{
    fs, io,
    path::{Path, PathBuf},
    string::String,
    vec::Vec,
};

use crate::agent::{AiDifficulty, Coordinate};
use crate::app::SavedGame;
use crate::engine::{GameState, SHIPS};

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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum AdapterState {
    Agent(AgentSnapshot),
    Ui(UiSnapshot),
    #[default]
    None,
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

/// File-backed active-save store for std runners.
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct FileSaveStore {
    path: PathBuf,
}

#[cfg(feature = "std")]
impl FileSaveStore {
    /// Create a file-backed store at `path`.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

/// Errors returned by [`FileSaveStore`].
#[cfg(feature = "std")]
#[derive(Debug)]
pub enum FileSaveError {
    Io(io::Error),
    Codec(Box<bincode::ErrorKind>),
}

#[cfg(feature = "std")]
impl core::fmt::Display for FileSaveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "save file I/O failed: {err}"),
            Self::Codec(err) => write!(f, "save file codec failed: {err}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FileSaveError {}

#[cfg(feature = "std")]
impl From<io::Error> for FileSaveError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(feature = "std")]
impl From<Box<bincode::ErrorKind>> for FileSaveError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        Self::Codec(err)
    }
}

#[cfg(feature = "std")]
impl SaveStore for FileSaveStore {
    type Error = FileSaveError;

    fn load_active(&mut self) -> Result<Option<SavedGame>, Self::Error> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                let mut game: SavedGame = bincode::deserialize(&bytes)?;
                restore_ship_names(&mut game);
                Ok(Some(game))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn save_active(&mut self, game: &SavedGame) -> Result<(), Self::Error> {
        let bytes = bincode::serialize(game)?;
        fs::write(&self.path, bytes)?;
        Ok(())
    }

    fn clear_active(&mut self) -> Result<(), Self::Error> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(feature = "std")]
fn restore_ship_names(game: &mut SavedGame) {
    restore_engine_ship_names(&mut game.local_engine);
    if let Some(engine) = &mut game.opponent_engine {
        restore_engine_ship_names(engine);
    }
}

#[cfg(feature = "std")]
fn restore_engine_ship_names(state: &mut GameState) {
    for (ship_state, ship) in state.my_board.ship_states.iter_mut().zip(SHIPS.iter()) {
        ship_state.name = ship.name();
    }
}
