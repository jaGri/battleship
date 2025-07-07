use std::fmt;

use crate::board::{Board, BoardError, GuessResult};
 
pub struct InterfaceCli<'a> {
    pub my_board: &'a Board,
    pub opponent_board: &'a Board,
}


impl <'a> InterfaceCli<'a> {
    pub fn new(my_board: &'a Board, opponent_board: &'a Board) -> Self {
        Self { my_board, opponent_board }
    }

    pub fn display(&self) {
        println!("================");
        self.display_boards();
    }

    pub fn display_boards(&self) {
        println!("My Board:");
        self.my_board.display();
        println!("\nOpponent's Board:");
        self.opponent_board.display();
    }

    pub fn make_guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> {
        self.opponent_board.guess(row, col)
    }
    
}