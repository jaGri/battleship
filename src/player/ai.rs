use crate::core::{
    ai,
    bitboard::BitBoard,
    board::Board,
    common::GuessResult,
    config::{BOARD_SIZE, NUM_SHIPS},
    BoardError,
};
use rand::rngs::SmallRng;

use super::Player;

/// Simple AI player that uses probability based guessing.
pub struct AiPlayer;

impl AiPlayer {
    pub fn new() -> Self {
        Self
    }
}

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

impl Player for AiPlayer {
    fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError> {
        for i in 0..NUM_SHIPS as usize {
            let (r, c, o) = board.random_placement(rng, i)?;
            board.place(i, r, c, o)?;
        }
        Ok(())
    }

    fn select_target(
        &mut self,
        rng: &mut SmallRng,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
    ) -> (usize, usize) {
        ai::calc_pdf_and_guess(hits, misses, remaining, rng)
    }

    fn handle_guess_result(&mut self, _coord: (usize, usize), _result: GuessResult) {}
    fn handle_opponent_guess(&mut self, _coord: (usize, usize), _result: GuessResult) {}
}
