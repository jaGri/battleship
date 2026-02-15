#![cfg(feature = "std")]

//! Experimental CLI helpers for displaying boards.
//! This module is not fully integrated and may be removed.
//! It is compiled only when the `std` feature is enabled.

use crate::core::{Board, BoardError, GuessResult};

#[allow(dead_code)]
pub struct InterfaceCli<'a> {
    pub my_board: &'a Board,
    pub opponent_board: &'a Board,
}

#[allow(dead_code)]
impl<'a> InterfaceCli<'a> {
    pub fn new(my_board: &'a Board, opponent_board: &'a Board) -> Self {
        Self {
            my_board,
            opponent_board,
        }
    }

    // TODO: Implement display methods when Board has display functionality
    // pub fn display(&self) { ... }
    // pub fn display_boards(&self) { ... }
    // pub fn make_guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> { ... }
}
