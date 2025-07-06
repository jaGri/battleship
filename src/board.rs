//! Game board state, using updated `BitBoard` and `Ship` types.

use core::fmt;
use rand::Rng;
use crate::config::{NUM_SHIPS, SHIPS, BOARD_SIZE};
use crate::bitboard::BitBoard;
use crate::ship::{Ship, Orientation};
use crate::common::{BoardError, GuessResult};

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
pub struct BoardState {
    ship_states: [ShipState; NUM_SHIPS as usize],
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
        let (_ship, mask) = Ship::<u128, { BOARD_SIZE as usize }>::new(def, orientation, row, col)?;
        // ensure no overlap
        if (self.ship_map & mask).count_ones() > 0 {
            return Err(BoardError::ShipOverlaps);
        }
        // record placement
        self.ship_map = self.ship_map | mask;
        self.ship_states[ship_index].name = def.name();
        // store ship in some external collection if needed
        Ok(())
    }

    /// Randomly place all ships without overlap.
    pub fn place_random<R: Rng>(&mut self, rng: &mut R) -> Result<(), BoardError> {
        for i in 0..NUM_SHIPS as usize {
            let def = SHIPS[i];
            loop {
                let orient = if rng.gen() {
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
                let r = rng.gen_range(0..=max_r);
                let c = rng.gen_range(0..=max_c);
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
        if self.hits.get(row, col).unwrap_or(false)
            || self.misses.get(row, col).unwrap_or(false)
        {
            return Err(BoardError::AlreadyGuessed);
        }
        // hit detection
        if self.ship_map.get(row, col).unwrap_or(false) {
            self.hits = self.hits | BitBoard::<u128, { BOARD_SIZE as usize }>::from_raw(
                (1u128 << (row * (BOARD_SIZE as usize) + col)),
            );
            // determine which ship
            for (i, def) in SHIPS.iter().enumerate() {
                // reconstruct mask for this ship placement
                // assume single placement per ship in ship_map
                let mask = Ship::<u128, { BOARD_SIZE as usize }>::new(*def, Orientation::Horizontal, row, col)
                    .map(|(_, m)| m)
                    .unwrap_or(BitBoard::new());
                if mask.get(row, col).unwrap_or(false) {
                    // register hit in state
                    // no hits tracked per ship in this simple model
                    return Ok(GuessResult::Hit);
                }
            }
            Ok(GuessResult::Hit)
        } else {
            self.misses = self.misses | BitBoard::<u128, { BOARD_SIZE as usize }>::from_raw(
                (1u128 << (row * (BOARD_SIZE as usize) + col)),
            );
            Ok(GuessResult::Miss)
        }
    }
}

impl fmt::Debug for BoardState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "BoardState {{\n  ship_map: {:?},\n  hits: {:?},\n  misses: {:?},\n  states: {:?}\n}}",
            self.ship_map, self.hits, self.misses, self.ship_states
        )
    }
}
