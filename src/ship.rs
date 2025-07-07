//! Ship definitions and placement logic using our `BitBoard`.

use core::fmt;
use num_traits::{PrimInt, Unsigned, Zero};

use crate::bitboard::BitBoard;
use crate::common::BoardError;

/// Orientation of a ship on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Public state of a ship on the board used for serialization or UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipState {
    pub name: &'static str,
    pub sunk: bool,
    pub position: Option<(usize, usize, Orientation)>,
}

impl ShipState {
    /// Create initial state for a ship.
    pub const fn new(name: &'static str) -> Self {
        ShipState {
            name,
            sunk: false,
            position: None,
        }
    }
}

impl<T, const N: usize> From<&Ship<T, N>> for ShipState
where
    T: PrimInt + Unsigned + Zero,
{
    fn from(ship: &Ship<T, N>) -> Self {
        ShipState {
            name: ship.ship_type().name(),
            sunk: ship.is_sunk(),
            position: Some((ship.row, ship.col, ship.orientation)),
        }
    }
}

impl<T, const N: usize> Ship<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    /// Construct a ship from a [`ShipState`] if placement information is present.
    pub fn from_state(state: &ShipState, def: ShipDef) -> Result<Option<Self>, BoardError> {
        if let Some((row, col, orient)) = state.position {
            Ok(Some(Ship::new(def, orient, row, col)?))
        } else {
            Ok(None)
        }
    }
}

/// Definition of a ship: name and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipDef {
    name: &'static str,
    length: usize,
}

impl ShipDef {
    /// Create a new ship definition.
    pub const fn new(name: &'static str, length: usize) -> Self {
        Self { name, length }
    }

    /// Ship's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Ship's length.
    pub fn length(&self) -> usize {
        self.length
    }
}

/// A ship placed on an N×N board, with hits tracked in a `BitBoard`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ship<T, const N: usize>
where
    T: PrimInt + Unsigned + Zero,
{
    ship_type: ShipDef,
    orientation: Orientation,
    row: usize,
    col: usize,
    mask: BitBoard<T, N>,
    hits: BitBoard<T, N>,
}

impl<T, const N: usize> Ship<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    /// Place a ship at (`row`, `col`) with `orientation`.
    /// Returns the newly constructed ship.
    pub fn new(
        ship_type: ShipDef,
        orientation: Orientation,
        row: usize,
        col: usize,
    ) -> Result<Self, BoardError> {
        let len = ship_type.length();
        // Ensure placement fits within N×N
        if orientation == Orientation::Horizontal {
            if col + len > N {
                return Err(BoardError::ShipOutOfBounds);
            }
        } else if row + len > N {
            return Err(BoardError::ShipOutOfBounds);
        }

        // Build occupancy mask
        let mut mask = BitBoard::<T, N>::new();
        for i in 0..len {
            let (r, c) = match orientation {
                Orientation::Horizontal => (row, col + i),
                Orientation::Vertical => (row + i, col),
            };
            mask.set(r, c)?;
        }

        // Initialize empty hits board
        let hits = BitBoard::<T, N>::new();
        Ok(Ship {
            ship_type,
            orientation,
            row,
            col,
            mask,
            hits,
        })
    }

    /// Register a hit at (`row`, `col`) using an occupancy mask.
    /// Returns `true` if it was a hit and records it.
    pub fn guess(&mut self, row: usize, col: usize) -> bool {
        if self.mask.get(row, col).unwrap_or(false) {
            let _ = self.hits.set(row, col);
            true
        } else {
            false
        }
    }

    /// Check if the ship is sunk (all segments hit).
    pub fn is_sunk(&self) -> bool {
        self.hits.count_ones() == self.ship_type.length()
    }

    /// Ship's type.
    pub fn ship_type(&self) -> ShipDef {
        self.ship_type
    }

    /// Origin of the ship (row, col).
    pub fn origin(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Orientation of the ship.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Occupancy mask of the ship on the board.
    pub fn mask(&self) -> BitBoard<T, N> {
        self.mask
    }
}

impl<T, const N: usize> fmt::Debug for Ship<T, N>
where
    T: PrimInt + Unsigned + Zero + fmt::Binary,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ship {{ name: \"{}\", origin: ({}, {}), orientation: {:?}, hits: {}, mask: {:?} }}",
            self.ship_type.name(),
            self.row,
            self.col,
            self.orientation,
            self.hits.count_ones(),
            self.mask,
        )
    }
}
