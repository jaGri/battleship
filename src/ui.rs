#![cfg(feature = "std")]

use crate::{
    bitboard::BitBoard,
    config::{BOARD_SIZE, NUM_SHIPS},
};
use rand::rngs::SmallRng;

// shorthand type for guess boards
pub type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Trait that allows the CLI to obtain AI suggestions without depending on the
/// AI module directly.
pub trait SuggestionProvider {
    /// Return a probability distribution along with a suggested guess. If no
    /// suggestion is available, return `None`.
    fn calc_pdf_and_guess(
        &mut self,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
        rng: &mut SmallRng,
    ) -> Option<(
        [[f64; BOARD_SIZE as usize]; BOARD_SIZE as usize],
        (usize, usize),
    )>;
}

/// Implementation of [`SuggestionProvider`] that uses the real AI logic.
pub struct AiSuggestion;

impl SuggestionProvider for AiSuggestion {
    fn calc_pdf_and_guess(
        &mut self,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
        rng: &mut SmallRng,
    ) -> Option<(
        [[f64; BOARD_SIZE as usize]; BOARD_SIZE as usize],
        (usize, usize),
    )> {
        let pdf = crate::ai::calc_pdf(hits, misses, remaining);
        let guess = crate::ai::sample_pdf(&pdf, 0.5, rng);
        Some((pdf, guess))
    }
}

/// [`SuggestionProvider`] that yields no suggestions.
pub struct NoSuggestion;

impl SuggestionProvider for NoSuggestion {
    fn calc_pdf_and_guess(
        &mut self,
        _hits: &BB,
        _misses: &BB,
        _remaining: &[usize; NUM_SHIPS as usize],
        _rng: &mut SmallRng,
    ) -> Option<(
        [[f64; BOARD_SIZE as usize]; BOARD_SIZE as usize],
        (usize, usize),
    )> {
        None
    }
}

/// Print a normalized probability distribution matrix.
pub fn print_probability_board(
    pdf: &[[f64; BOARD_SIZE as usize]; BOARD_SIZE as usize],
) {
    std::println!("\nProbability distribution:");
    std::print!("   ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {:>4}", ch);
    }
    std::println!();
    for r in 0..BOARD_SIZE as usize {
        std::print!("{:2} ", r + 1);
        for c in 0..BOARD_SIZE as usize {
            std::print!(" {:4.2}", pdf[r][c]);
        }
        std::println!();
    }
}

