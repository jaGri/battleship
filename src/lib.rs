#![no_std]

#[cfg(feature = "std")]
extern crate std;
extern crate alloc;

mod ai;
mod bitboard;
mod board;
mod protocol;
mod common;
mod config;
mod game;
mod player;
mod player_ai;
mod player_cli;
mod ship;
mod transport;
#[cfg(feature = "std")]
mod skeleton;
#[cfg(feature = "std")]
mod transport_tcp;
//mod interface_cli;

pub use ai::*;
pub use bitboard::{BitBoard, BitBoardError};
pub use board::*;
pub use common::*;
pub use config::*;
pub use protocol::*;
pub use game::*;
pub use player::*;
pub use player_ai::*;
pub use player_cli::*;
pub use ship::*;
pub use transport::*;
#[cfg(feature = "std")]
pub use skeleton::*;
#[cfg(feature = "std")]
pub use transport_tcp::*;
//pub use interface_cli::*;
