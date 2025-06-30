use crate::{bitboard::{BitBoard, BitBoardError, Orientation}, board};
use std::fmt;
use rand::{rand_core::impls, Rng};

pub const BOARD_SIZE: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipType {
    name: &'static str,
    size: usize,
}

pub const NUM_SHIPS: usize = 5;
pub const SHIPS: [ShipType; NUM_SHIPS] = [
    ShipType {
        name: "Carrier",
        size: 5,
    },
    ShipType {
        name: "Battleship",
        size: 4,
    },
    ShipType {
        name: "Cruiser",
        size: 3,
    },
    ShipType {
        name: "Submarine",
        size: 3,
    },
    ShipType {
        name: "Destroyer",
        size: 2,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ship {
    name: &'static str,
    size: usize,
    bitboard: BitBoard<u128>,
    hits: BitBoard<u128>,
    placed: bool,
    sunk: bool,
}

impl Ship {
    fn is_sunk(&self) -> bool {
        self.hits.value() == self.bitboard.value()
    }
}


pub struct ShipState {
    name: &'static str,
    sunk: bool,
}

impl ShipState {
    pub fn new(name: &'static str, sunk: bool) -> Self {
        Self { name, sunk }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn sunk(&self) -> bool {
        self.sunk
    }
}

pub struct BoardState {
    ship_states: [ShipState; NUM_SHIPS],
    hits_coords: BitBoard<u128>,
    miss_coords: BitBoard<u128>,
    ship_coords: Option<BitBoard<u128>>,
}

impl BoardState {
    pub fn new(board: &Board, incl_ships: bool) -> Result<Self, BoardError> {
        let ship_states: [ShipState; NUM_SHIPS] = board
            .ships
            .iter()
            .map(|ship| ShipState {name: ship.name, sunk: ship.sunk,})
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| BoardError::InvalidIndex)?;
        Ok(Self {
            ship_states,
            hits_coords: board.hits()?.clone(),
            miss_coords: board.misses().clone(),
            ship_coords: if incl_ships {
                Some(board.ships()?)
            } else {
                None
            },
        })
    }
}

pub enum GuessResult {
    Hit,
    Miss,
    Sink(&'static str), // Ship name
}

/// Errors for Board operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BoardError {
    // BitBoard errors, such as invalid grid size.
    BitBoardError(BitBoardError),
    /// Requested grid size is zero, too large (>255), or exceeds `T` capacity.
    NameNotFound,
    // Index is invalid or out of bounds for ship array.
    InvalidIndex,
    // Ship already placed
    ShipAlreadyPlaced,
    // Ship can't overlap with another ship
    ShipOverlaps,
    // Guess already made at this position
    AlreadyGuessed,
    // Unable to place ship
    UnableToPlaceShip,
}



pub struct Board {
    size: usize,
    ships: [Ship; NUM_SHIPS],
    misses: BitBoard<u128>,
}

impl Board {
    pub fn new(board_size: usize) -> Result<Self, BitBoardError> {
        // Initialize empty bitboard and verify it fits the requested size
        let empty_bb = BitBoard::new(board_size)?;
        // Initialize ships with their specifications
        let ships: [Ship; NUM_SHIPS] = SHIPS.map(|spec| Ship {
            name: spec.name,
            size: spec.size,
            bitboard: empty_bb,
            hits: empty_bb,
            placed: false,
            sunk: false,
        });
        Ok(Self {
            size: board_size as usize,
            ships,
            misses: empty_bb,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn misses(&self) -> &BitBoard<u128> {
        &self.misses
    }

    pub fn hits(&self) -> Result<BitBoard<u128>, BoardError> {
        let mut bb = BitBoard::new(self.size()).map_err(BoardError::BitBoardError)?;
        // Collect all hits from all ships
        for ship in &self.ships {
            bb = bb | ship.hits; // Bitwise OR to combine hits
        }
        Ok(bb)
    }

    pub fn guesses(&self) -> Result<BitBoard<u128>, BoardError> {
        Ok(*self.misses() | self.hits()?)
    }

    pub fn state(&self, incl_ships:bool) -> Result<BoardState, BoardError> {
        BoardState::new(self, incl_ships)
    }

    pub fn ships(&self) -> Result<BitBoard<u128>, BoardError> {
        let mut bb = BitBoard::new(self.size()).map_err(BoardError::BitBoardError)?;
        // Collect all ship placements
        for ship in &self.ships {
            if ship.placed {
                bb = bb | ship.bitboard; // Bitwise OR to combine ship placements
            }
        }
        Ok(bb) 
    }

    fn get_ship_index(&self, ship_name: &str) -> Result<usize, BoardError> {
        SHIPS
            .iter()
            .position(|ship| ship.name == ship_name)
            .ok_or(BoardError::NameNotFound)
    }

    fn calc_placement(&self, size: usize, row: usize, col: usize, orientation: Orientation) -> Result<BitBoard<u128>, BoardError> {
        let mut bb = BitBoard::new(self.size()).map_err(BoardError::BitBoardError)?;
        bb.fill(row, col, orientation, size, true).map_err(BoardError::BitBoardError)?;
        Ok(bb)
    }

    pub fn place(
        &mut self,
        ship_name: &str,
        row: usize,
        col: usize,
        orientation: Orientation,
    ) -> Result<(), BoardError> {
        let ship_index = self.get_ship_index(ship_name)?;
        if ship_index >= NUM_SHIPS {
            return Err(BoardError::InvalidIndex);
        }
        if self.ships[ship_index].placed {
            return Err(BoardError::ShipAlreadyPlaced);
        }
        let ship_size = self.ships[ship_index].size;
        let bb = self.calc_placement(ship_size, row, col, orientation)?;
        // Check if the ship overlaps with any already placed ships
        let overlaps: bool = self
            .ships
            .iter()
            .enumerate()
            // skip the ship we’re placing, and any that aren’t placed yet
            .filter(|&(i, ship)| i != ship_index && ship.placed)
            // check if any of them overlap
            .any(|(_, ship)| {
                // use the BitAnd impl; if any bit in common then non‐zero
                bb.intersects(&ship.bitboard).unwrap_or(false)
            });
        if overlaps {
            return Err(BoardError::ShipOverlaps);
        }
        // Fill the bitboard for the ship
        let ship: &mut Ship = &mut self.ships[ship_index];
        ship.bitboard = bb;
        ship.placed = true;
        Ok(())
    }

    pub fn place_random(&mut self, ship_name: &str) -> Result<(), BoardError> {
        let ship_index = self.get_ship_index(ship_name)?;
        if ship_index >= NUM_SHIPS {
            return Err(BoardError::InvalidIndex);
        }
        if self.ships[ship_index].placed {
            return Err(BoardError::ShipAlreadyPlaced);
        }
        let mut placed: bool = false;
        let mut rng = rand::rng();
        let max_attempts = 1000;
        let mut attempts = 0;
        while !placed && attempts < max_attempts {
            // Randomly choose row, column, and orientation
            let row = rng.random_range(0..self.size);
            let col = rng.random_range(0..self.size);
            let orientation: Orientation = if rng.random_bool(0.5) {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            };
            // Try to place the ship
            match self.place(ship_name, row, col, orientation) {
                Ok(_) => placed = true,
                Err(BoardError::ShipOverlaps) => {
                    attempts += 1;
                    continue;
                }
                Err(e) => return Err(e), // Other errors are returned immediately
            }
        }
        if placed {
            Ok(())
        } else {
            Err(BoardError::UnableToPlaceShip)
        }
    }

    pub fn guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> {
        let guesses = self.guesses()?;
        if guesses.get(row, col).map_err(BoardError::BitBoardError)? {
            return Err(BoardError::AlreadyGuessed);
        }
        // Check each ship to see if it was hit
        for i in 0..self.ships.len() {
            let ship = &mut self.ships[i];
            // Check if the ship has already been sunk
            if ship.sunk {
                continue;
            }
            // Check if the ship is hit
            if ship.bitboard.get(row, col).map_err(BoardError::BitBoardError)? {
                // If the guess hits a ship, mark a hit on both the ship and the board
                ship.hits.set(row, col, true).map_err(BoardError::BitBoardError)?;
                // Check if the ship is now sunk
                if ship.is_sunk() {
                    ship.sunk = true;
                    return Ok(GuessResult::Sink(ship.name));
                } else {
                    return Ok(GuessResult::Hit);
                }
            }
        }
        // Missed
        self.misses.set(row, col, true).map_err(BoardError::BitBoardError)?;
        Ok(GuessResult::Miss)
    }
}
