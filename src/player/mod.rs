//! Player trait and implementations
//!
//! This module defines the Player trait and provides concrete implementations:
//! - AiPlayer: Probability-based AI using hunt/target mode
//! - CliPlayer: Interactive command-line player
//! - PlayerNode: Orchestrator combining Player + GameEngine + Transport

use crate::core::{
    bitboard::BitBoard,
    board::Board,
    common::GuessResult,
    config::{BOARD_SIZE, NUM_SHIPS},
    BoardError,
};
use rand::rngs::SmallRng;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Interface implemented by different player types.
///
/// A Player is responsible for:
/// - Placing ships on the board
/// - Selecting targets to attack
/// - Handling feedback from guesses
pub trait Player: Send {
    /// Place all ships onto the provided board.
    fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError>;

    /// Choose the next target coordinate given guess history and remaining enemy ships.
    fn select_target(
        &mut self,
        rng: &mut SmallRng,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
    ) -> (usize, usize);

    /// Inform the player of the result of its last guess.
    fn handle_guess_result(&mut self, _coord: (usize, usize), _result: GuessResult) {}

    /// Inform the player of an opponent guess against its board.
    fn handle_opponent_guess(&mut self, _coord: (usize, usize), _result: GuessResult) {}
}

// Re-export implementations
pub mod ai;
pub use ai::AiPlayer;

#[cfg(feature = "std")]
pub mod cli;
#[cfg(feature = "std")]
pub use cli::CliPlayer;

#[cfg(feature = "std")]
pub mod node;
#[cfg(feature = "std")]
pub use node::PlayerNode;
