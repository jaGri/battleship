use std::fmt;

use crate::board::{Board, BoardError, GuessResult};
 
pub struct InterfaceCli<'a> {
    pub my_board: &'a Board,
    pub opponent_board: &'a Board,
}

impl fmt::Display for BoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            BoardError::NameNotFound => "Unknown ship name",
            BoardError::InvalidIndex => "Invalid ship index",
            BoardError::ShipAlreadyPlaced => "Ship already placed",
            BoardError::ShipOverlaps => "Ship overlaps with another ship",
            BoardError::BitBoardError(e) => &e.to_string(),
            BoardError::AlreadyGuessed => "Guess already made at this position",
            BoardError::UnableToPlaceShip => "Unable to place ship",
        };
        write!(f, "{}", msg)
    }
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