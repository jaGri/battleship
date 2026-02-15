//! Core battleship game engine (no_std compatible)
//!
//! This module contains the pure game logic with zero external dependencies
//! (except num-traits, libm, rand::alloc). It can be used in embedded systems
//! or compiled to WebAssembly.

pub mod ai;
pub mod bitboard;
pub mod board;
pub mod common;
pub mod config;
pub mod game;
pub mod ship;

// Re-export commonly used types
pub use ai::{calc_pdf, calc_pdf_and_guess, sample_pdf};
pub use bitboard::{BitBoard, BitBoardError};
pub use board::{Board, BoardState};
pub use common::{BoardError, GuessResult};
pub use config::*;
pub use game::{GameEngine, GameState, GameStatus, GuessBoardState};
pub use ship::{Orientation, Ship, ShipDef, ShipState};
