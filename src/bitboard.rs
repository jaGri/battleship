//! A generic, fixed-size bitboard for N×N grid-based applications.
//!
//! Stores each cell as a bit in a primitive integer type `T`. Supports setting/clearing individual cells or ranges,
//! combining boards with bitwise ops, and checking overlaps via `&`. Includes idiomatic trait implementations and unit tests.

use num_traits::{PrimInt, ToPrimitive};
use std::fmt;
use std::ops::{BitAnd, BitOr};

/// Errors for BitBoard operations.
#[derive(Debug, PartialEq, Eq)]
pub enum BitBoardError {
    /// Requested grid size is zero, too large (>255), or exceeds `T` capacity.
    InvalidGridSize,
    /// Row or column index out of bounds.
    InvalidIndex,
    /// Two boards of different sizes cannot be combined.
    SizeMismatch,
    /// Failed to convert integer to u128 for popcount.
    ConversionError
}

impl fmt::Display for BitBoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            BitBoardError::InvalidGridSize => "Invalid grid size",
            BitBoardError::InvalidIndex    => "Row or column index out of bounds",
            BitBoardError::SizeMismatch    => "Two boards of different sizes cannot be compared.",
            BitBoardError::ConversionError => "Failed to convert integer",
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for BitBoardError {}

/// Direction for drawing a contiguous line of bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A fixed-size N×N bitboard packed into an integer type `T`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitBoard<T>
where
    T: PrimInt + fmt::Binary,
{
    bits: T,
    size: usize,
    max_bits: usize,
}


impl<T> BitBoard<T>
where
    T: PrimInt + fmt::Binary + ToPrimitive,
{
    /// Creates a new size×size board with all bits cleared.
    pub fn new(size: usize) -> Result<Self, BitBoardError> {
        if size == 0 || size > 255 {
            return Err(BitBoardError::InvalidGridSize);
        }
        let max = T::zero().count_zeros() as usize;
        if size.checked_mul(size).map_or(true, |area| area > max) {
            return Err(BitBoardError::InvalidGridSize);
        }
        Ok(Self { bits: T::zero(), size, max_bits: max })
    }

    /// Retrieves the board dimension N.
    pub fn size(&self) -> usize { self.size }

    /// Retrieves the integer representation of the board.
    pub fn value(&self) -> T { self.bits }

    /// Gets bit by linear index.
    fn get_at(&self, idx: usize) -> Result<bool, BitBoardError> {
        if idx >= self.max_bits {
            Err(BitBoardError::InvalidIndex)
        } else {
            let mask = T::one().shl(idx);
            Ok(self.bits & mask != T::zero())
        }
    }

    /// Gets bit at (row, col).
    pub fn get(&self, row: usize, col: usize) -> Result<bool, BitBoardError> {
        let idx = self.idx(row, col)?;
        self.get_at(idx)
    }

    /// Sets or clears bit by linear index.
    fn set_at(&mut self, idx: usize, on: bool) -> Result<(), BitBoardError> {
        if idx >= self.max_bits {
            return Err(BitBoardError::InvalidIndex);
        }
        let mask = T::one().shl(idx);
        self.bits = if on { self.bits | mask } else { self.bits & !mask };
        Ok(())
    }

    /// Sets or clears bit at (row, col).
    pub fn set(&mut self, row: usize, col: usize, on: bool) -> Result<(), BitBoardError> {
        let idx = self.idx(row, col)?;
        self.set_at(idx, on)
    }

    /// Fills a line of length `len` at (row, col) in given orientation.
    pub fn fill(
        &mut self,
        row: usize,
        col: usize,
        orient: Orientation,
        len: usize,
        on: bool,
    ) -> Result<(), BitBoardError> {
        if len == 0 { return Err(BitBoardError::InvalidIndex); }
        match orient {
            Orientation::Horizontal => {
                if col + len > self.size { return Err(BitBoardError::InvalidIndex); }
                for c in col..col+len { self.set(row, c, on)?; }
            }
            Orientation::Vertical => {
                if row + len > self.size { return Err(BitBoardError::InvalidIndex); }
                for r in row..row+len { self.set(r, col, on)?; }
            }
        }
        Ok(())
    }

    /// Maps (row, col) to linear index, validating bounds.
    fn idx(&self, row: usize, col: usize) -> Result<usize, BitBoardError> {
        if row >= self.size || col >= self.size {
            Err(BitBoardError::InvalidIndex)
        } else {
            Ok(row * self.size + col)
        }
    }

    /// Checks if the board instersects with another board.
    pub fn intersects(&self, other: &Self) -> Result<bool, BitBoardError> {
        if self.size != other.size {
            return Err(BitBoardError::SizeMismatch);
        }
        Ok((self.bits & other.bits) != T::zero())
    }

    /// Counts the number of bits set to `1` in the board.
    pub fn count_ones(&self) -> Result<usize, BitBoardError> {
        // Convert underlying integer to u128 for popcount
        match self.bits.to_u128() {
            Some(v) => Ok(v.count_ones() as usize),
            None => Err(BitBoardError::ConversionError),
        }
    }


}

/// Display board as grid of 0/1.
impl<T> fmt::Display for BitBoard<T>
where
    T: PrimInt + fmt::Binary,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in 0..self.size {
            for c in 0..self.size {
                let ch = if self.get(r, c).unwrap_or(false) { '1' } else { '0' };
                write!(f, "{} ", ch)?;
            }
            if r+1<self.size { writeln!(f)?; }
        }
        Ok(())
    }
}

/// Default = 10×10 or fallback to max-fitting.
impl<T> Default for BitBoard<T>
where
    T: PrimInt + fmt::Binary,
{
    fn default() -> Self {
        const D: usize = 10;
        BitBoard::new(D).unwrap_or_else(|_| {
            let max = T::zero().count_zeros() as usize;
            let fallback = (max as f64).sqrt().floor() as usize;
            BitBoard::new(fallback).expect("fallback valid")
        })
    }
}

/// Overlay using `|` (panics on size mismatch).
impl<T> BitOr for BitBoard<T>
where
    T: PrimInt + fmt::Binary,
{
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        assert_eq!(self.size, rhs.size);
        BitBoard { bits: self.bits | rhs.bits, size: self.size, max_bits: self.max_bits }
    }
}

/// Intersection using `&` (panics on size mismatch).
impl<T> BitAnd for BitBoard<T>
where
    T: PrimInt + fmt::Binary,
{
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        assert_eq!(self.size, rhs.size);
        BitBoard { bits: self.bits & rhs.bits, size: self.size, max_bits: self.max_bits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_new() {
        assert_eq!(BitBoard::<u8>::new(0).unwrap_err(), BitBoardError::InvalidGridSize);
        assert_eq!(BitBoard::<u8>::new(20).unwrap_err(), BitBoardError::InvalidGridSize);
    }

    #[test]
    fn set_get() {
        let mut b = BitBoard::<u16>::new(4).unwrap();
        assert!(!b.get(1,1).unwrap());
        b.set(1,1,true).unwrap();
        assert!(b.get(1,1).unwrap());
        b.set(1,1,false).unwrap();
        assert!(!b.get(1,1).unwrap());
    }

    #[test]
    fn fill_tests() {
        let mut b = BitBoard::<u16>::new(4).unwrap();
        b.fill(0,0,Orientation::Horizontal,3,true).unwrap();
        assert!(b.get(0,2).unwrap());
        b.fill(1,1,Orientation::Vertical,2,true).unwrap();
        assert!(b.get(2,1).unwrap());
    }

    #[test]
    fn bitwise_and_or() {
        let mut a = BitBoard::<u16>::new(4).unwrap();
        let mut c = a;
        a.set(0,0,true).unwrap(); c.set(0,1,true).unwrap();
        let o = a | c; assert!(o.get(0,0).unwrap() && o.get(0,1).unwrap());
        let i = a & c; assert!(!i.get(0,0).unwrap());
    }
}
