//! Probability-based AI guessing using marginal placement counting.
//!
//! This module implements a fast O(ships × placements) algorithm that computes
//! the probability that each cell contains a ship by counting how many valid
//! placements of each remaining ship would cover that cell.
//!
//! Key features:
//! - Marginal counting: O(ships × placements) ≈ 4000 operations
//! - Weighted hit targeting: placements covering hits receive bonus weight
//! - Sunk ship tracking: excludes identified sunk ship cells from calculations
//! - Temperature-based sampling: controls exploration vs exploitation
//! - no_std compatible: no heap allocations

use super::{
    bitboard::BitBoard,
    config::{BOARD_SIZE, NUM_SHIPS},
    ship::Orientation,
};
use libm::pow;
use rand::Rng;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;
const GRID_SIZE: usize = BOARD_SIZE as usize;

/// Compute probability distribution over unguessed cells using marginal counting.
///
/// For each remaining ship, this function enumerates all valid placements and
/// counts how many times each cell would be covered. Placements that cover
/// unsunk hit cells receive bonus weighting to focus the AI on targeting hits.
///
/// # Algorithm
/// 1. For each remaining ship (by length):
///    - Try all possible placements (row, col, orientation)
///    - Skip placements overlapping misses or sunk ships
///    - Count how many unsunk hits the placement covers
///    - Apply weight = 1 + hits_covered * hit_weight
///    - Add weight to all cells in the placement
/// 2. Normalize to convert counts into probabilities
///
/// # Parameters
/// - `hits`: All hit cells (includes sunk ships)
/// - `misses`: All miss cells
/// - `sunk`: Cells belonging to fully identified sunk ships
/// - `remaining_lengths`: Lengths of unsunk ships (0 for sunk ships)
/// - `hit_weight`: Bonus weight per unsunk hit covered (0 = no bonus, 10 = strong targeting)
///
/// # Returns
/// A 10×10 matrix of probabilities (0.0 to 1.0) for each cell.
pub fn calc_pdf(
    hits: &BB,
    misses: &BB,
    sunk: &BB,
    remaining_lengths: &[usize; NUM_SHIPS as usize],
    hit_weight: f64,
) -> [[f64; GRID_SIZE]; GRID_SIZE] {
    let mut matrix = [[0.0f64; GRID_SIZE]; GRID_SIZE];

    // Cells that are blocked for new placements (misses + sunk ships)
    let blocked = misses.into_raw() | sunk.into_raw();
    // Hits not yet attributed to a sunk ship
    let unsunk_hits = hits.into_raw() & !sunk.into_raw();

    // Marginal counting: enumerate placements for each remaining ship
    for &len in remaining_lengths.iter() {
        if len == 0 {
            continue; // Ship already sunk
        }

        for orient in [Orientation::Horizontal, Orientation::Vertical] {
            let (max_row, max_col) = match orient {
                Orientation::Vertical => (GRID_SIZE - len + 1, GRID_SIZE),
                Orientation::Horizontal => (GRID_SIZE, GRID_SIZE - len + 1),
            };

            for r in 0..max_row {
                for c in 0..max_col {
                    // Build placement mask and check validity
                    let mut placement_mask = 0u128;
                    let mut hits_covered = 0;

                    for k in 0..len {
                        let (rr, cc) = match orient {
                            Orientation::Vertical => (r + k, c),
                            Orientation::Horizontal => (r, c + k),
                        };
                        let cell_bit = 1u128 << (rr * GRID_SIZE + cc);
                        placement_mask |= cell_bit;

                        // Check if placement overlaps blocked cells
                        if (blocked & cell_bit) != 0 {
                            placement_mask = 0; // Mark invalid
                            break;
                        }

                        // Count unsunk hits covered
                        if (unsunk_hits & cell_bit) != 0 {
                            hits_covered += 1;
                        }
                    }

                    // Skip invalid placements
                    if placement_mask == 0 {
                        continue;
                    }

                    // Calculate weight: linear bonus for covering hits
                    // Weight = 1 + hits_covered * hit_weight
                    // Example: hit_weight=10, 1 hit → 11×, 2 hits → 21×
                    let weight = 1.0 + (hits_covered as f64) * hit_weight;

                    // Add weight to all cells in this placement
                    let mut bits = placement_mask;
                    while bits != 0 {
                        let idx = bits.trailing_zeros() as usize;
                        let (rr, cc) = (idx / GRID_SIZE, idx % GRID_SIZE);

                        // Only accumulate probability for unguessed cells
                        if !hits.get(rr, cc).unwrap_or(false)
                            && !misses.get(rr, cc).unwrap_or(false)
                        {
                            matrix[rr][cc] += weight;
                        }

                        bits &= bits - 1; // Clear lowest bit
                    }
                }
            }
        }
    }

    normalize(matrix)
}

fn normalize(mut matrix: [[f64; GRID_SIZE]; GRID_SIZE]) -> [[f64; GRID_SIZE]; GRID_SIZE] {
    let mut total = 0.0;
    for row in matrix.iter() {
        for &v in row.iter() {
            total += v;
        }
    }
    if total == 0.0 {
        let uniform = 1.0 / (GRID_SIZE * GRID_SIZE) as f64;
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                matrix[r][c] = uniform;
            }
        }
        return matrix;
    }
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            matrix[r][c] /= total;
        }
    }
    matrix
}

/// Sample a coordinate from a probability matrix using temperature-controlled sampling.
///
/// Temperature controls how greedily the AI exploits the probability distribution:
/// - Low temperature (0.1-0.3): Strongly favors high-probability cells (greedy)
/// - Temperature = 1.0: Samples proportional to raw probabilities
/// - High temperature (2.0+): More uniform, exploratory sampling
///
/// Uses Boltzmann sampling: P_adjusted ∝ P^(1/T)
pub fn sample_pdf<R: Rng + ?Sized>(
    pdf: &[[f64; GRID_SIZE]; GRID_SIZE],
    temperature: f64,
    rng: &mut R,
) -> (usize, usize) {
    // Clamp temperature to reasonable range to prevent numerical instability
    let temp = temperature.max(0.01).min(10.0);

    let mut adjusted = [[0.0f64; GRID_SIZE]; GRID_SIZE];
    let mut total = 0.0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            let v = pow(pdf[r][c], 1.0 / temp);
            adjusted[r][c] = v;
            total += v;
        }
    }

    // Fallback if all probabilities are zero
    if total == 0.0 {
        return (
            rng.random_range(0..GRID_SIZE),
            rng.random_range(0..GRID_SIZE),
        );
    }

    // Weighted sampling
    let threshold: f64 = rng.random_range(0.0..total);
    let mut cumulative = 0.0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            cumulative += adjusted[r][c];
            if threshold < cumulative {
                return (r, c);
            }
        }
    }

    // Fallback (should rarely hit due to floating point precision)
    (GRID_SIZE - 1, GRID_SIZE - 1)
}

/// Convenience wrapper that calculates the PDF and immediately selects a guess.
///
/// # Parameters
/// - `temperature`: Controls sampling greediness (0.1 = greedy, 2.0 = exploratory)
/// - `hit_weight`: Bonus weight per unsunk hit covered (0-20 typical range)
pub fn calc_pdf_and_guess<R: Rng + ?Sized>(
    hits: &BB,
    misses: &BB,
    sunk: &BB,
    lengths: &[usize; NUM_SHIPS as usize],
    rng: &mut R,
    temperature: f64,
    hit_weight: f64,
) -> (usize, usize) {
    let pdf = calc_pdf(hits, misses, sunk, lengths, hit_weight);
    sample_pdf(&pdf, temperature, rng)
}
