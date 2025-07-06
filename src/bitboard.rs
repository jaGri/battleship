//! A fixed-size bitboard implementation using const generics.
//!
//! The type is `no_std` friendly and avoids heap allocations. Boards are
//! represented as an `N×N` grid packed into an unsigned integer `T`.
//! Basic constructors and bitwise operations are provided.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use core::{any, fmt, mem};
use num_traits::{PrimInt, Unsigned, Zero};

/// Errors returned by bitboard operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBoardError {
    /// Requested board size N*N exceeds capacity of `T::BITS`.
    SizeTooLarge { n: usize, capacity: usize },
    /// Row or column index is out of bounds [0..N).
    IndexOutOfBounds { row: usize, col: usize },
}

impl core::fmt::Display for BitBoardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BitBoardError::SizeTooLarge { n, capacity } => {
                write!(
                    f,
                    "SizeTooLarge: N*N={} exceeds T::BITS={}",
                    n * n,
                    capacity
                )
            }
            BitBoardError::IndexOutOfBounds { row, col } => {
                write!(f, "IndexOutOfBounds: row={}, col={}", row, col)
            }
        }
    }
}

/// A fixed-size N×N bitboard stored in the unsigned integer `T`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BitBoard<T, const N: usize>
where
    T: PrimInt + Unsigned + Zero,
{
    bits: T,
}

impl<T, const N: usize> BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    /// Number of usable bits in the board (`N * N`).
    const BOARD_BITS: usize = N * N;

    #[inline]
    fn mask() -> T {
        if Self::BOARD_BITS == mem::size_of::<T>() * 8 {
            !T::zero()
        } else {
            (T::one() << Self::BOARD_BITS) - T::one()
        }
    }

    /// Create a new empty bitboard (all bits cleared) without size check.
    #[inline]
    pub fn new() -> Self {
        BitBoard { bits: T::zero() }
    }

    /// Fallible constructor: returns `Err(SizeTooLarge)` if N*N > T::BITS.
    pub fn try_new() -> Result<Self, BitBoardError> {
        let capacity = mem::size_of::<T>() * 8;
        if Self::BOARD_BITS > capacity {
            Err(BitBoardError::SizeTooLarge { n: N, capacity })
        } else {
            Ok(BitBoard { bits: T::zero() })
        }
    }

    /// Returns the number of set bits (occupied cells).
    pub fn count_ones(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Returns true if no bits are set.
    pub fn is_empty(&self) -> bool {
        self.bits.is_zero()
    }

    /// Gets the bit at (row, col).
    pub fn get(&self, row: usize, col: usize) -> Result<bool, BitBoardError> {
        self.check_bounds(row, col)?;
        let idx = row * N + col;
        Ok(((self.bits >> idx) & T::one()) != T::zero())
    }

    /// Sets the bit at (row, col) to 1.
    pub fn set(&mut self, row: usize, col: usize) -> Result<(), BitBoardError> {
        self.check_bounds(row, col)?;
        let idx = row * N + col;
        self.bits = self.bits | (T::one() << idx);
        Ok(())
    }

    /// Clears the bit at (row, col) to 0.
    pub fn clear(&mut self, row: usize, col: usize) -> Result<(), BitBoardError> {
        self.check_bounds(row, col)?;
        let idx = row * N + col;
        self.bits = self.bits & !(T::one() << idx);
        Ok(())
    }

    /// Toggles the bit at (row, col).
    pub fn toggle(&mut self, row: usize, col: usize) -> Result<(), BitBoardError> {
        self.check_bounds(row, col)?;
        let idx = row * N + col;
        self.bits = self.bits ^ (T::one() << idx);
        Ok(())
    }

    /// Sets all board bits to `1`.
    #[inline]
    pub fn fill(&mut self) {
        self.bits = Self::mask();
    }

    /// Clears all bits to `0`.
    #[inline]
    pub fn clear_all(&mut self) {
        self.bits = T::zero();
    }

    #[inline]
    fn check_bounds(&self, row: usize, col: usize) -> Result<(), BitBoardError> {
        if row >= N || col >= N {
            Err(BitBoardError::IndexOutOfBounds { row, col })
        } else {
            Ok(())
        }
    }

    /// Consumes the board and returns the raw integer.
    #[inline]
    pub fn into_raw(self) -> T {
        self.bits
    }

    /// Creates a bitboard from the raw integer, masking out upper bits.
    #[inline]
    pub fn from_raw(raw: T) -> Self {
        BitBoard {
            bits: raw & Self::mask(),
        }
    }

    /// Creates a bitboard from an iterator over `(row, col)` positions.
    #[inline]
    pub fn from_iter<I>(iter: I) -> Result<Self, BitBoardError>
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        let mut board = Self::new();
        for (r, c) in iter {
            board.set(r, c)?;
        }
        Ok(board)
    }

    /// Iterator over the set bits of the board.
    #[inline]
    pub fn iter_set_bits(&self) -> SetBits<'_, T, N> {
        SetBits {
            board: self,
            idx: 0,
        }
    }
}

impl<T, const N: usize> Default for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> fmt::Debug for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero + fmt::Binary,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BitBoard<{}, {}>:", any::type_name::<T>(), N)?;
        for r in 0..N {
            for c in 0..N {
                let bit = if ((self.bits >> (r * N + c)) & T::one()) != T::zero() {
                    '■'
                } else {
                    '□'
                };
                write!(f, "{} ", bit)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<T, const N: usize> fmt::Display for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero + fmt::Binary,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in 0..N {
            for c in 0..N {
                let bit = if ((self.bits >> (r * N + c)) & T::one()) != T::zero() {
                    '■'
                } else {
                    '□'
                };
                write!(f, "{} ", bit)?;
            }
            if r + 1 < N {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// Iterator over the set bits of a bitboard.
#[derive(Clone, Copy)]
pub struct SetBits<'a, T, const N: usize>
where
    T: PrimInt + Unsigned + Zero,
{
    board: &'a BitBoard<T, N>,
    idx: usize,
}

impl<'a, T, const N: usize> Iterator for SetBits<'a, T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    type Item = (usize, usize);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < N * N {
            let idx = self.idx;
            self.idx += 1;
            if ((self.board.bits >> idx) & T::one()) != T::zero() {
                return Some((idx / N, idx % N));
            }
        }
        None
    }
}

/// Macro for compile-time assertion of size and creation.
#[macro_export]
macro_rules! bitboard {
    ($T:ty, $N:expr) => {{
        const _ASSERT: [(); 1] = [(); ($N * $N <= core::mem::size_of::<$T>() * 8) as usize];
        let _ = _ASSERT;
        $crate::BitBoard::<$T, $N>::new()
    }};
}

/// Bitwise AND for combining two bitboards.
impl<T, const N: usize> BitAnd for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        BitBoard::from_raw(self.into_raw() & rhs.into_raw())
    }
}

/// Bitwise OR for combining two bitboards.
impl<T, const N: usize> BitOr for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        BitBoard::from_raw(self.into_raw() | rhs.into_raw())
    }
}

/// Bitwise XOR for combining two bitboards.
impl<T, const N: usize> BitXor for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        BitBoard::from_raw(self.into_raw() ^ rhs.into_raw())
    }
}

/// Bitwise NOT for inverting a bitboard (within board bounds).
impl<T, const N: usize> Not for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        Self::from_raw(!self.bits)
    }
}

impl<T, const N: usize> BitAndAssign for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.bits = self.bits & rhs.bits;
    }
}

impl<T, const N: usize> BitOrAssign for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits = self.bits | rhs.bits;
    }
}

impl<T, const N: usize> BitXorAssign for BitBoard<T, N>
where
    T: PrimInt + Unsigned + Zero,
{
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.bits = self.bits ^ rhs.bits;
    }
}

/// Convenience aliases for common board sizes.
pub mod aliases {
    use super::BitBoard;

    /// 8×8 board in `u64`.
    pub type BB8x8 = BitBoard<u64, 8>;
    /// 4×4 board in `u16`.
    pub type BB4x4 = BitBoard<u16, 4>;
    /// N×N board in `u8`.
    pub type BB8<const N: usize> = BitBoard<u8, N>;
    /// N×N board in `u16`.
    pub type BB16<const N: usize> = BitBoard<u16, N>;
    /// N×N board in `u32`.
    pub type BB32<const N: usize> = BitBoard<u32, N>;
    /// N×N board in `u64`.
    pub type BB64<const N: usize> = BitBoard<u64, N>;
    /// N×N board in `u128`.
    pub type BB128<const N: usize> = BitBoard<u128, N>;
}
