#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// ========================================
// Layer 1: Core game engine (no_std)
// ========================================
pub mod core;

// Convenience re-exports of core types
pub use core::{
    ai::*, bitboard::*, board::*, common::*, config::*, game::*, ship::*,
};

// ========================================
// Layer 2: Player & Transport
// ========================================
pub mod player;
pub use player::{AiPlayer, Player};

#[cfg(feature = "std")]
pub use player::{CliPlayer, PlayerNode};

#[cfg(feature = "std")]
pub mod transport;

#[cfg(feature = "std")]
pub use transport::tcp::TcpTransport;
#[cfg(feature = "std")]
pub use transport::heartbeat::HeartbeatTransport;

// ========================================
// Layer 3: Protocol & RPC
// ========================================
#[cfg(feature = "std")]
pub mod protocol;

#[cfg(feature = "std")]
pub use protocol::{domain, GameApi, Message, Skeleton, Stub, PROTOCOL_VERSION};

// ========================================
// Layer 4: CLI
// ========================================
#[cfg(feature = "std")]
pub mod cli;
