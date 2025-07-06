//! Game board state, using updated `BitBoard` and `Ship` types.

use crate::bitboard::BitBoard;
use crate::common::{BoardError, GuessResult};
use crate::config::{BOARD_SIZE, NUM_SHIPS, SHIPS};
use crate::ship::{Orientation, Ship};
use core::fmt;
use rand::Rng;

/// Tracks per-ship state: name and sunk flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipState {
    pub name: &'static str,
    pub sunk: bool,
}
impl ShipState {
    /// Create initial state for a ship.
    pub const fn new(name: &'static str) -> Self {
        ShipState { name, sunk: false }
    }
}

/// Main board state: ship placements, hits, misses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlacedShip {
    ship: Ship<u128, { BOARD_SIZE as usize }>,
}

pub struct BoardState {
    ship_states: [ShipState; NUM_SHIPS as usize],
    ships: [Option<PlacedShip>; NUM_SHIPS as usize],
    ship_map: BitBoard<u128, { BOARD_SIZE as usize }>,
    hits: BitBoard<u128, { BOARD_SIZE as usize }>,
    misses: BitBoard<u128, { BOARD_SIZE as usize }>,
}

impl BoardState {
    /// Create an empty board state (no ships placed).
    pub fn new() -> Self {
        // initialize ship states from config names
        let ship_states = core::array::from_fn(|i: usize| {
            let def = SHIPS[i];
            ShipState::new(def.name())
        });
        let empty = BitBoard::<u128, { BOARD_SIZE as usize }>::new();
        BoardState {
            ship_states,
            ships: [None; NUM_SHIPS as usize],
            ship_map: empty,
            hits: empty,
            misses: empty,
        }
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
        let def = SHIPS[ship_index];
        let ship = Ship::<u128, { BOARD_SIZE as usize }>::new(def, orientation, row, col)?;
        let mask = ship.mask();
        // ensure no overlap
        if (self.ship_map & mask).count_ones() > 0 {
            return Err(BoardError::ShipOverlaps);
        }
        // record placement
        self.ship_map = self.ship_map | mask;
        self.ship_states[ship_index].name = def.name();
        self.ships[ship_index] = Some(PlacedShip { ship });
        Ok(())
    }

    /// Randomly place all ships without overlap.
    pub fn place_random<R: Rng>(&mut self, rng: &mut R) -> Result<(), BoardError> {
        for i in 0..NUM_SHIPS as usize {
            let def = SHIPS[i];
            loop {
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
                if self.place(i, r, c, orient).is_ok() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Process a guess at (row, col), marking hits/misses and reporting result.
    pub fn guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> {
        // prevent duplicates
        if self.hits.get(row, col).unwrap_or(false) || self.misses.get(row, col).unwrap_or(false) {
            return Err(BoardError::AlreadyGuessed);
        }
        // hit detection
        if self.ship_map.get(row, col).unwrap_or(false) {
            let bit = BitBoard::<u128, { BOARD_SIZE as usize }>::from_raw(
                1u128 << (row * (BOARD_SIZE as usize) + col),
            );
            self.hits = self.hits | bit;

            // determine which ship was hit
            for (i, slot) in self.ships.iter_mut().enumerate() {
                if let Some(ps) = slot {
                    if ps.ship.mask().get(row, col).unwrap_or(false) {
                        ps.ship.register_hit(row, col);
                        if ps.ship.is_sunk() && !self.ship_states[i].sunk {
                            self.ship_states[i].sunk = true;
                            return Ok(GuessResult::Sink(ps.ship.ship_type().name()));
                        }
                        return Ok(GuessResult::Hit);
                    }
                }
            }
            // should have found a ship; fallback
            Ok(GuessResult::Hit)
        } else {
            self.misses = self.misses
                | BitBoard::<u128, { BOARD_SIZE as usize }>::from_raw(
                    1u128 << (row * (BOARD_SIZE as usize) + col),
                );
            Ok(GuessResult::Miss)
        }
    }
}

impl fmt::Debug for BoardState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "BoardState {{\n  ship_map: {:?},\n  hits: {:?},\n  misses: {:?},\n  states: {:?},\n  ships: {:?}\n}}",
            self.ship_map,
            self.hits,
            self.misses,
            self.ship_states,
            self.ships
        )
    }
}
