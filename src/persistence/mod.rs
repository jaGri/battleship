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

#[cfg(feature = "std")]
const SAVE_MAGIC: &[u8; 4] = b"BSAV";
#[cfg(feature = "std")]
const SAVE_VERSION: u8 = 1;
#[cfg(feature = "std")]
const SAVE_LENGTH_BYTES: usize = 8;
#[cfg(feature = "std")]
const SAVE_MAC_BYTES: usize = 32;
#[cfg(feature = "std")]
const SAVE_HEADER_BYTES: usize =
    SAVE_MAGIC.len() + core::mem::size_of::<u8>() + SAVE_LENGTH_BYTES + SAVE_MAC_BYTES;
#[cfg(feature = "std")]
const SAVE_AUTH_KEY: [u8; 32] = *b"battleship local save auth key!!";

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
    Integrity(SaveIntegrityError),
}

/// Integrity errors returned while authenticating a file save envelope.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveIntegrityError {
    Header,
    Version,
    Length,
    Mac,
}

#[cfg(feature = "std")]
impl core::fmt::Display for FileSaveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "save file I/O failed: {err}"),
            Self::Codec(err) => write!(f, "save file codec failed: {err}"),
            Self::Integrity(err) => write!(f, "save file integrity check failed: {err}"),
        }
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for SaveIntegrityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Header => write!(f, "invalid header"),
            Self::Version => write!(f, "unsupported version"),
            Self::Length => write!(f, "invalid payload length"),
            Self::Mac => write!(f, "authentication tag mismatch"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FileSaveError {}

#[cfg(feature = "std")]
impl std::error::Error for SaveIntegrityError {}

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
                let payload = open_save_envelope(&bytes)?;
                let mut game: SavedGame = bincode::deserialize(payload)?;
                restore_ship_names(&mut game);
                Ok(Some(game))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn save_active(&mut self, game: &SavedGame) -> Result<(), Self::Error> {
        let payload = bincode::serialize(game)?;
        let bytes = seal_save_envelope(&payload)?;
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
fn seal_save_envelope(payload: &[u8]) -> Result<Vec<u8>, FileSaveError> {
    let payload_len = u64::try_from(payload.len())
        .map_err(|_| FileSaveError::Integrity(SaveIntegrityError::Length))?;
    let length_bytes = payload_len.to_be_bytes();
    let mac = save_mac(&length_bytes, payload);

    let mut bytes = Vec::with_capacity(SAVE_HEADER_BYTES + payload.len());
    bytes.extend_from_slice(SAVE_MAGIC);
    bytes.push(SAVE_VERSION);
    bytes.extend_from_slice(&length_bytes);
    bytes.extend_from_slice(mac.as_bytes());
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

#[cfg(feature = "std")]
fn open_save_envelope(bytes: &[u8]) -> Result<&[u8], FileSaveError> {
    if bytes.len() < SAVE_HEADER_BYTES || &bytes[..SAVE_MAGIC.len()] != SAVE_MAGIC {
        return Err(FileSaveError::Integrity(SaveIntegrityError::Header));
    }

    let version_offset = SAVE_MAGIC.len();
    if bytes[version_offset] != SAVE_VERSION {
        return Err(FileSaveError::Integrity(SaveIntegrityError::Version));
    }

    let length_offset = version_offset + core::mem::size_of::<u8>();
    let mac_offset = length_offset + SAVE_LENGTH_BYTES;
    let payload_offset = mac_offset + SAVE_MAC_BYTES;

    let length_bytes: [u8; SAVE_LENGTH_BYTES] = bytes[length_offset..mac_offset]
        .try_into()
        .map_err(|_| FileSaveError::Integrity(SaveIntegrityError::Length))?;
    let payload_len = u64::from_be_bytes(length_bytes);
    let payload_len = usize::try_from(payload_len)
        .map_err(|_| FileSaveError::Integrity(SaveIntegrityError::Length))?;
    if bytes.len() != payload_offset + payload_len {
        return Err(FileSaveError::Integrity(SaveIntegrityError::Length));
    }

    let expected_mac = save_mac(&length_bytes, &bytes[payload_offset..]);
    let actual_mac = &bytes[mac_offset..payload_offset];
    if !mac_eq(expected_mac.as_bytes(), actual_mac) {
        return Err(FileSaveError::Integrity(SaveIntegrityError::Mac));
    }

    Ok(&bytes[payload_offset..])
}

#[cfg(feature = "std")]
fn save_mac(length_bytes: &[u8; SAVE_LENGTH_BYTES], payload: &[u8]) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new_keyed(&SAVE_AUTH_KEY);
    hasher.update(SAVE_MAGIC);
    hasher.update(&[SAVE_VERSION]);
    hasher.update(length_bytes);
    hasher.update(payload);
    hasher.finalize()
}

#[cfg(feature = "std")]
fn mac_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
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
        ship_state.length = ship.length();
    }
    for (ship_state, ship) in state.my_guesses.ships.iter_mut().zip(SHIPS.iter()) {
        ship_state.name = ship.name();
        ship_state.length = ship.length();
    }
}
