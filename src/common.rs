#![no_std]
//! Common types for Battleship: board errors and guess results.

use crate::bitboard::BitBoardError;

/// Result of a guess attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    /// Guess hit an undepleted ship segment.
    Hit,
    /// Guess missed all ships.
    Miss,
    /// Guess sank a ship, carrying its name.
    Sink(&'static str),
}

/// Errors returned by Board operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BoardError {
    /// Underlying bitboard error (e.g., invalid size or index).
    BitBoardError(BitBoardError),
    /// Named ship not found in configuration.
    NameNotFound,
    /// Specified index is out of range.
    InvalidIndex,
    /// Attempted to place a ship that is already placed.
    ShipAlreadyPlaced,
    /// Ship placement overlaps another ship.
    ShipOverlaps,
    /// Guess was already made at this position.
    AlreadyGuessed,
    /// Unable to place ship (random or manual placement failed).
    UnableToPlaceShip,
    // Ship out of bounds
    ShipOutOfBounds
}

impl From<BitBoardError> for BoardError {
    fn from(err: BitBoardError) -> Self {
        BoardError::BitBoardError(err)
    }
}
impl core::fmt::Display for BoardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BoardError::BitBoardError(e) => write!(f, "BitBoard error: {}", e),
            BoardError::NameNotFound => write!(f, "Ship name not found in configuration"),
            BoardError::InvalidIndex => write!(f, "Index is out of range"),
            BoardError::ShipAlreadyPlaced => write!(f, "Ship is already placed on the board"),
            BoardError::ShipOverlaps => write!(f, "Ship placement overlaps with another ship"),
            BoardError::AlreadyGuessed => write!(f, "Guess was already made at this position"),
            BoardError::UnableToPlaceShip => write!(f, "Unable to place ship"),
            BoardError::ShipOutOfBounds => write!(f, "Ship placement is out of bounds"),
        }
    }
}