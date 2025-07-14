#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;
mod ai;
mod bitboard;
mod board;
pub mod domain;
mod common;
mod config;
mod game;
mod player;
mod player_ai;
#[cfg(feature = "std")]
mod player_cli;
pub mod protocol;
pub mod transport;
#[cfg(feature = "std")]
pub mod transport_tcp;
mod ship;
pub mod skeleton;
pub mod stub;
pub mod player_node;
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
pub use player_cli::*;
pub use protocol::*;
pub use ship::*;
pub use skeleton::*;
pub use stub::*;
pub use player_node::*;
//pub use interface_cli::*;
