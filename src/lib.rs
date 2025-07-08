#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;
mod ai;
mod bitboard;
mod board;
mod common;
mod config;
mod game;
mod player;
mod player_ai;
#[cfg(feature = "std")]
mod player_cli;
mod ship;
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
pub use ship::*;
//pub use interface_cli::*;
