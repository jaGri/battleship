// Probability-based guessing logic for the opponent board.
// Uses no_std and avoids heap allocations.

use crate::{
    bitboard::BitBoard,
    config::{BOARD_SIZE, NUM_SHIPS},
    ship::Orientation,
};
use rand::Rng;

/// Bitboard type alias for convenience.
type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

const GRID_SIZE: usize = BOARD_SIZE as usize;

/// Compute a probability density over all unguessed squares given the sets of
/// known hits and misses and the lengths of remaining enemy ships. The result
/// is a matrix where each entry sums the relative likelihood of a ship segment
/// occupying that coordinate.
pub fn calc_pdf(
    hits: &BB,
    misses: &BB,
    remaining_lengths: &[usize; NUM_SHIPS as usize],
) -> [[f64; GRID_SIZE]; GRID_SIZE] {
    let mut matrix = [[0.0f64; GRID_SIZE]; GRID_SIZE];

    for &len in remaining_lengths.iter() {
        if len == 0 { continue; }

        for orient in [Orientation::Horizontal, Orientation::Vertical] {
            let max_row = if matches!(orient, Orientation::Vertical) {
                GRID_SIZE - len + 1
            } else {
                GRID_SIZE
            };
            let max_col = if matches!(orient, Orientation::Horizontal) {
                GRID_SIZE - len + 1
            } else {
                GRID_SIZE
            };
            for r in 0..max_row {
                for c in 0..max_col {
                    // check placement validity and count hits
                    let mut valid = true;
                    let mut n_hits = 0usize;
                    for k in 0..len {
                        let rr = r + if matches!(orient, Orientation::Vertical) { k } else {0};
                        let cc = c + if matches!(orient, Orientation::Horizontal) { k } else {0};
                        if misses.get(rr, cc).unwrap_or(false) {
                            valid = false;
                            break;
                        }
                        if hits.get(rr, cc).unwrap_or(false) {
                            n_hits += 1;
                        }
                    }
                    if !valid { continue; }

                    // Placements covering more observed hits should receive
                    // dramatically more weight so that squares adjacent to
                    // confirmed hits stand out. The previous implementation
                    // used a base of 2 which diluted the impact when many
                    // other placements were possible. Using a larger bias
                    // concentrates the probability mass around known hits.
                    const HIT_BIAS: f64 = 10.0;
                    let weight = if n_hits == 0 {
                        1.0
                    } else {
                        HIT_BIAS.powi(n_hits as i32)
                    };
                    for k in 0..len {
                        let rr = r + if matches!(orient, Orientation::Vertical) { k } else {0};
                        let cc = c + if matches!(orient, Orientation::Horizontal) { k } else {0};
                        if !hits.get(rr, cc).unwrap_or(false) && !misses.get(rr, cc).unwrap_or(false) {
                            matrix[rr][cc] += weight;
                        }
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

/// Sample a coordinate from a probability matrix using a temperature parameter.
pub fn sample_pdf<R: Rng + ?Sized>(
    pdf: &[[f64; GRID_SIZE]; GRID_SIZE],
    temperature: f64,
    rng: &mut R,
) -> (usize, usize) {
    let mut adjusted = [[0.0f64; GRID_SIZE]; GRID_SIZE];
    let mut total = 0.0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            let v = pdf[r][c].powf(1.0 / temperature);
            adjusted[r][c] = v;
            total += v;
        }
    }
    if total == 0.0 {
        return (
            rng.random_range(0..GRID_SIZE),
            rng.random_range(0..GRID_SIZE),
        );
    }
    let mut cumulative = 0.0;
    let threshold: f64 = rng.random_range(0.0..total);
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            cumulative += adjusted[r][c];
            if threshold < cumulative {
                return (r, c);
            }
        }
    }
    (GRID_SIZE - 1, GRID_SIZE - 1)
}

/// Convenience wrapper that calculates the PDF and immediately selects a guess
/// using the provided random number generator.
pub fn calc_pdf_and_guess<R: Rng + ?Sized>(
    hits: &BB,
    misses: &BB,
    lengths: &[usize; NUM_SHIPS as usize],
    rng: &mut R,
) -> (usize, usize) {
    let pdf = calc_pdf(hits, misses, lengths);
    // Lower temperature biases the sampling towards higher probability cells
    // so suggestions hone in on likely ship locations.
    sample_pdf(&pdf, 0.5, rng)
}

