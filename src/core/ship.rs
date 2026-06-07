//! Ship definitions and placement logic using our `BitBoard`.

use core::fmt;
use num_traits::{PrimInt, Unsigned, Zero};

use super::bitboard::BitBoard;
use super::common::BoardError;

/// Orientation of a ship on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Orientation {
    Horizontal,
    Vertical,
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
    pub const fn length(&self) -> usize {
        self.length
    }
}

/// Shared ship state for owned ships and revealed enemy ships.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Ship<T, const N: usize>
where
    T: PrimInt + Unsigned + Zero,
{
    #[cfg_attr(feature = "std", serde(skip))]
    pub name: &'static str,
    pub length: usize,
    pub sunk: bool,
    pub hits: BitBoard<T, N>,
    pub placement: BitBoard<T, N>,
    pub position: Option<(usize, usize, Orientation)>,
}

impl<T, const N: usize> Ship<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    /// Create an unknown or unplaced ship slot.
    pub fn unknown(ship_type: ShipDef) -> Self {
        Self {
            name: ship_type.name(),
            length: ship_type.length(),
            sunk: false,
            hits: BitBoard::<T, N>::new(),
            placement: BitBoard::<T, N>::new(),
            position: None,
        }
    }

    /// Place a ship at (`row`, `col`) with `orientation`.
    pub fn new(
        ship_type: ShipDef,
        orientation: Orientation,
        row: usize,
        col: usize,
    ) -> Result<Self, BoardError> {
        let len = ship_type.length();
        if orientation == Orientation::Horizontal {
            if col + len > N {
                return Err(BoardError::ShipOutOfBounds);
            }
        } else if row + len > N {
            return Err(BoardError::ShipOutOfBounds);
        }

        let mut placement = BitBoard::<T, N>::new();
        for i in 0..len {
            let (r, c) = match orientation {
                Orientation::Horizontal => (row, col + i),
                Orientation::Vertical => (row + i, col),
            };
            placement.set(r, c)?;
        }

        Ok(Self {
            name: ship_type.name(),
            length: ship_type.length(),
            sunk: false,
            hits: BitBoard::<T, N>::new(),
            placement,
            position: Some((row, col, orientation)),
        })
    }

    /// Restore the canonical static definition after deserialization.
    pub fn with_definition(mut self, ship_type: ShipDef) -> Self {
        self.name = ship_type.name();
        self.length = ship_type.length();
        self
    }

    /// Returns true when this ship has a known placement.
    pub fn is_placed(&self) -> bool {
        self.position.is_some() && !self.placement.is_empty()
    }

    /// Returns true when the known placement covers (`row`, `col`).
    pub fn covers(&self, row: usize, col: usize) -> bool {
        self.placement.get(row, col).unwrap_or(false)
    }

    /// Register a hit at (`row`, `col`).
    pub fn record_hit(&mut self, row: usize, col: usize) -> bool {
        if self.covers(row, col) {
            let _ = self.hits.set(row, col);
            self.sunk = self.hits.count_ones() == self.length;
            true
        } else {
            false
        }
    }

    /// Backward-friendly alias for recording a hit on a placed ship.
    pub fn guess(&mut self, row: usize, col: usize) -> bool {
        self.record_hit(row, col)
    }

    /// Check if the ship is sunk.
    pub fn is_sunk(&self) -> bool {
        self.sunk
    }

    /// Ship's type.
    pub fn ship_type(&self) -> ShipDef {
        ShipDef::new(self.name, self.length)
    }

    /// Origin of the ship (row, col), or `(0, 0)` when unknown.
    pub fn origin(&self) -> (usize, usize) {
        self.position
            .map(|(row, col, _)| (row, col))
            .unwrap_or((0, 0))
    }

    /// Orientation of the ship, or horizontal when unknown.
    pub fn orientation(&self) -> Orientation {
        self.position
            .map(|(_, _, orientation)| orientation)
            .unwrap_or(Orientation::Horizontal)
    }

    /// Occupancy mask of known ship cells.
    pub fn mask(&self) -> BitBoard<T, N> {
        self.placement
    }

    /// Ship length if still afloat, otherwise zero.
    pub fn remaining_length(&self) -> usize {
        if self.sunk {
            0
        } else {
            self.length
        }
    }

    /// Reveal this ship as sunk with a known footprint.
    pub fn reveal_sunk(&mut self, footprint: BitBoard<T, N>) {
        self.placement = footprint;
        self.hits = footprint;
        self.sunk = true;
        self.position = None;
    }
}

impl<T, const N: usize> fmt::Debug for Ship<T, N>
where
    T: PrimInt + Unsigned + Zero + fmt::Binary,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ship {{ name: \"{}\", length: {}, sunk: {}, position: {:?}, hits: {}, placement: {:?} }}",
            self.name,
            self.length,
            self.sunk,
            self.position,
            self.hits.count_ones(),
            self.placement,
        )
    }
}
