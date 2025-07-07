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
pub struct BoardState {
    pub ship_states: [ShipState; NUM_SHIPS as usize],
    pub ships: [Option<Ship<u128, { BOARD_SIZE as usize }>>; NUM_SHIPS as usize],
    pub ship_map: BB,
    pub hits: BB,
    pub misses: BB,
}

/// Main board state: ship placements, hits, misses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlacedShip {
    ship: Ship<u128, { BOARD_SIZE as usize }>,
}

pub struct Board {
    ship_states: [ShipState; NUM_SHIPS as usize],
    ships: [Option<PlacedShip>; NUM_SHIPS as usize],
    ship_map: BB,
    hits: BB,
    misses: BB,
}

impl Board {
    /// Create an empty board state (no ships placed).
    pub fn new() -> Self {
        // initialize ship states from config names
        let ship_states = core::array::from_fn(|i: usize| {
            let def = SHIPS[i];
            ShipState::new(def.name())
        });
        let empty = BB::new();
        Board {
            ship_states,
            ships: [None; NUM_SHIPS as usize],
            ship_map: empty,
            hits: empty,
            misses: empty,
        }
    }

    /// Immutable view of ship states.
    pub fn ship_states(&self) -> &[ShipState] {
        &self.ship_states
    }

    /// Returns `true` when all ships are sunk.
    pub fn all_sunk(&self) -> bool {
        self.ship_states.iter().all(|s| s.sunk)
    }

    /// Board occupancy mask of all ships.
    pub fn ship_map(&self) -> BB {
        self.ship_map
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
        self.ship_states[ship_index].name = def.name();
        self.ships[ship_index] = Some(PlacedShip { ship });
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
        // prevent duplicates
        if self.hits.get(row, col)? || self.misses.get(row, col)? {
            return Err(BoardError::AlreadyGuessed);
        }
        // hit detection
        if self.ship_map.get(row, col)? {
            self.hits.set(row, col)?;

            // determine which ship was hit
            for (i, slot) in self.ships.iter_mut().enumerate() {
                if let Some(ps) = slot {
                    if ps.ship.mask().get(row, col).unwrap_or(false) {
                        ps.ship.guess(row, col);
                        if ps.ship.is_sunk() && !self.ship_states[i].sunk {
                            self.ship_states[i].sunk = true;
                            return Ok(GuessResult::Sink(ps.ship.ship_type().name()));
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
            "Board {{\n  ship_map: {:?},\n  hits: {:?},\n  misses: {:?},\n  states: {:?},\n  ships: {:?}\n}}",
            self.ship_map,
            self.hits,
            self.misses,
            self.ship_states,
            self.ships
        )
    }
}

impl From<&Board> for BoardState {
    fn from(b: &Board) -> Self {
        let ships = core::array::from_fn(|i| b.ships[i].map(|ps| ps.ship));
        BoardState {
            ship_states: b.ship_states,
            ships,
            ship_map: b.ship_map,
            hits: b.hits,
            misses: b.misses,
        }
    }
}

impl From<BoardState> for Board {
    fn from(state: BoardState) -> Self {
        let ships = core::array::from_fn(|i| state.ships[i].map(|s| PlacedShip { ship: s }));
        Board {
            ship_states: state.ship_states,
            ships,
            ship_map: state.ship_map,
            hits: state.hits,
            misses: state.misses,
        }
    }
}
