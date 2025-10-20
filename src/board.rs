//! Game board state, using updated `BitBoard` and `Ship` types.

use crate::bitboard::BitBoard;
use crate::common::{BoardError, GuessResult};
use crate::config::{BOARD_SIZE, NUM_SHIPS, SHIPS};
use crate::ship::{Orientation, Ship, ShipState};
use core::fmt;
use rand::Rng;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Serializable board state for syncing or saving games.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BoardState {
    pub ship_states: [ShipState; NUM_SHIPS as usize],
    pub ship_map: BB,
    pub hits: BB,
    pub misses: BB,
}

/// Main board state: ship placements, hits, misses.

pub struct Board {
    ships: [Option<Ship<u128, { BOARD_SIZE as usize }>>; NUM_SHIPS as usize],
    ship_map: BB,
    hits: BB,
    misses: BB,
}

impl Board {
    /// Create an empty board state (no ships placed).
    pub fn new() -> Self {
        let empty = BB::new();
        Board {
            ships: [None; NUM_SHIPS as usize],
            ship_map: empty,
            hits: empty,
            misses: empty,
        }
    }

    /// Returns the public state of each ship.
    pub fn ship_states(&self) -> [ShipState; NUM_SHIPS as usize] {
        core::array::from_fn(|i| match &self.ships[i] {
            Some(s) => ShipState {
                name: s.ship_type().name(),
                sunk: s.is_sunk(),
                position: Some((s.origin().0, s.origin().1, s.orientation())),
            },
            None => ShipState::new(SHIPS[i].name()),
        })
    }

    /// Returns `true` when all ships are sunk.
    pub fn all_sunk(&self) -> bool {
        self.ships.iter().enumerate().all(|(i, s)| match s {
            Some(ship) => ship.is_sunk(),
            None => {
                let _ = SHIPS[i];
                false
            }
        })
    }

    /// Board occupancy mask of all ships.
    pub fn ship_map(&self) -> BB {
        self.ship_map
    }

    /// Bitboard of hits recorded on this board.
    pub fn hits(&self) -> BB {
        self.hits
    }

    /// Bitboard of misses recorded on this board.
    pub fn misses(&self) -> BB {
        self.misses
    }

    /// Place a single ship by index at (row, col) and orientation.
    pub fn place(
        &mut self,
        ship_index: usize,
        row: usize,
        col: usize,
        orientation: Orientation,
    ) -> Result<(), BoardError> {
        if ship_index >= NUM_SHIPS as usize {
            return Err(BoardError::InvalidIndex);
        }
        if self.ships[ship_index].is_some() {
            return Err(BoardError::ShipAlreadyPlaced);
        }
        let def = SHIPS[ship_index];
        let ship = Ship::<u128, { BOARD_SIZE as usize }>::new(def, orientation, row, col)?;
        let mask = ship.mask();
        // ensure no overlap
        if !(self.ship_map & mask).is_empty() {
            return Err(BoardError::ShipOverlaps);
        }
        // record placement
        self.ship_map = self.ship_map | mask;
        self.ships[ship_index] = Some(ship);
        Ok(())
    }

    /// Returns a random non‐overlapping (row, col, Orientation) for `ship_index`.
    pub fn random_placement<R: Rng>(
        &self,
        rng: &mut R,
        ship_index: usize,
    ) -> Result<(usize, usize, Orientation), BoardError> {
        if ship_index >= NUM_SHIPS as usize {
            return Err(BoardError::InvalidIndex);
        }
        let def = SHIPS[ship_index];
        let mut attempts = 0;
        while attempts < 100 {
            attempts += 1;
            let orient = if rng.random() {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            };
            let max_r = if orient == Orientation::Vertical {
                BOARD_SIZE as usize - def.length()
            } else {
                BOARD_SIZE as usize - 1
            };
            let max_c = if orient == Orientation::Horizontal {
                BOARD_SIZE as usize - def.length()
            } else {
                BOARD_SIZE as usize - 1
            };
            let r = rng.random_range(0..=max_r);
            let c = rng.random_range(0..=max_c);
            // build a temp ship and check overlap
            let ship = Ship::<u128, { BOARD_SIZE as usize }>::new(def, orient, r, c)?;
            if (self.ship_map & ship.mask()).is_empty() {
                return Ok((r, c, orient));
            }
        }
        Err(BoardError::UnableToPlaceShip)
    }

    /// Process a guess at (row, col), marking hits/misses and reporting result.
    pub fn guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> {
        // bounds check via BitBoard::get
        let already_hit = self.hits.get(row, col)?;
        let already_miss = self.misses.get(row, col)?;

        // prevent duplicates
        if self.hits.get(row, col)? || self.misses.get(row, col)? {
            return Err(BoardError::AlreadyGuessed);
        }
        // hit detection
        if self.ship_map.get(row, col)? {
            self.hits.set(row, col)?;

            // determine which ship was hit
            for ship_opt in self.ships.iter_mut() {
                if let Some(ship) = ship_opt {
                    if ship.mask().get(row, col).unwrap_or(false) {
                        let was_sunk = ship.is_sunk();
                        ship.guess(row, col);
                        if ship.is_sunk() && !was_sunk {
                            return Ok(GuessResult::Sink(ship.ship_type().name()));
                        }
                        return Ok(GuessResult::Hit);
                    }
                }
            }
            // should have found a ship; fallback
            Err(BoardError::UnknownShipHit)
        } else {
            self.misses.set(row, col)?;
            Ok(GuessResult::Miss)
        }
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Board {{\n  ship_map: {:?},\n  hits: {:?},\n  misses: {:?},\n  ships: {:?}\n}}",
            self.ship_map, self.hits, self.misses, self.ships
        )
    }
}

impl From<&Board> for BoardState {
    fn from(b: &Board) -> Self {
        BoardState {
            ship_states: b.ship_states(),
            ship_map: b.ship_map,
            hits: b.hits,
            misses: b.misses,
        }
    }
}

impl From<BoardState> for Board {
    fn from(state: BoardState) -> Self {
        let mut board = Board::new();
        board.ship_map = state.ship_map;
        board.hits = state.hits;
        board.misses = state.misses;
        for i in 0..NUM_SHIPS as usize {
            let def = SHIPS[i];
            if let Some(ship) =
                Ship::<u128, { BOARD_SIZE as usize }>::from_state(&state.ship_states[i], def)
                    .unwrap()
            {
                board.ships[i] = Some(ship);
            }
        }
        board
    }
}
