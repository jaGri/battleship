#![no_std]
mod common;
mod bitboard;
mod board;
mod ship;
mod config;
mod game;
//mod interface_cli;

pub use bitboard::{BitBoard, BitBoardError};
pub use common::*;
pub use board::*;
pub use ship::*;
pub use config::*;
pub use game::*;
//pub use interface_cli::*;
