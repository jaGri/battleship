#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// ========================================
// Layer 1: Core game engine (no_std)
// ========================================
pub mod core;
pub use crate::core as engine;

// Convenience re-exports of core types
pub use engine::{ai::*, bitboard::*, board::*, common::*, config::*, game::*, ship::*};

// ========================================
// Layer 2: App adapters & Transport
// ========================================
pub mod agent;
pub mod app;
pub mod input;
pub mod render;

#[cfg(feature = "std")]
pub mod render_cli;

pub use agent::{
    AiAgent, AiDifficulty, Difficulty, HumanAgent, PlayerAgent, RemoteAgent, ScriptedAgent,
};
pub use app::{AppCommand, AppEvent, AppState, BattleshipApp, MatchState, SavedGame};
pub use input::{InputSource, UiEvent};
#[cfg(feature = "persistence")]
pub use persistence::{AdapterState, AgentSnapshot, MemorySaveStore, SaveStore, UiSnapshot};
pub use render::{Renderer, ScreenView};

#[cfg(feature = "std")]
pub use render_cli::{CliInput, CliRenderer};

pub mod transport;

#[cfg(feature = "std")]
pub use transport::heartbeat::HeartbeatTransport;
#[cfg(feature = "in-memory")]
pub use transport::in_memory::InMemoryTransport;
#[cfg(feature = "tcp")]
pub use transport::tcp::TcpTransport;

#[cfg(feature = "persistence")]
pub mod persistence;

#[cfg(feature = "data-generation")]
pub mod data_generation;

// ========================================
// Layer 3: Protocol & RPC
// ========================================
pub mod protocol;

pub use protocol::{domain, WireMessage, PROTOCOL_VERSION};
