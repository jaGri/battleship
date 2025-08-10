#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
mod ai;
mod bitboard;
mod board;
mod common;
mod config;
pub mod domain;
mod game;
mod player;
mod player_ai;
#[cfg(feature = "std")]
mod logging;
#[cfg(feature = "std")]
mod player_cli;
#[cfg(feature = "std")]
pub mod player_node;
pub mod protocol;
mod ship;
pub mod skeleton;
pub mod stub;
#[cfg(feature = "std")]
pub mod transport;
//mod interface_cli;

pub use ai::*;
pub use bitboard::{BitBoard, BitBoardError};
pub use board::*;
pub use common::*;
pub use config::*;
pub use game::*;
pub use player::*;
pub use player_ai::*;
#[cfg(feature = "std")]
pub use logging::init_logging;
#[cfg(feature = "std")]
pub use player_cli::*;
#[cfg(feature = "std")]
pub use player_node::*;
pub use protocol::*;
pub use ship::*;
#[cfg(feature = "std")]
pub use skeleton::*;
#[cfg(feature = "std")]
pub use stub::*;
#[cfg(feature = "std")]
pub use transport::tcp::TcpTransport;
//pub use interface_cli::*;
