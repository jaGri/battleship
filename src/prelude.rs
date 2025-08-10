//! Commonly used types and utilities for ease of import.

pub use crate::{calc_pdf, ship_name_static, AiPlayer, GameEngine, GameStatus, Player};

#[cfg(feature = "std")]
pub use crate::{print_player_view, print_probability_board, CliPlayer, PlayerNode};

#[cfg(feature = "std")]
pub use crate::transport::{in_memory::InMemoryTransport, tcp::TcpTransport, Transport};
