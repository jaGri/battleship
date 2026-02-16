use crate::core::{
    ai,
    bitboard::BitBoard,
    board::Board,
    common::GuessResult,
    config::{BOARD_SIZE, NUM_SHIPS, SHIPS},
    BoardError,
};
use rand::rngs::SmallRng;

use super::Player;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Difficulty level for AI players.
///
/// Controls AI strength through temperature (decision greediness) and
/// hit_weight (targeting aggressiveness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(clap::ValueEnum))]
pub enum Difficulty {
    /// High exploration, weak targeting. Makes many suboptimal choices.
    Easy,
    /// Balanced play with moderate exploration.
    Medium,
    /// Strong play with mostly greedy decisions (default).
    Hard,
    /// Near-optimal greedy play with aggressive hit targeting.
    Expert,
}

/// AI player using marginal placement counting with sunk ship tracking.
///
/// This player tracks which hit cells belong to sunk ships and excludes them
/// from probability calculations, leading to more accurate targeting.
pub struct AiPlayer {
    /// Bitboard of cells belonging to fully identified sunk ships.
    sunk: BB,
    /// Temperature for sampling (lower = greedier, higher = more random).
    temperature: f64,
    /// Weight bonus per unsunk hit covered by a placement.
    hit_weight: f64,
    /// Track the last guess to identify sunk ship cells.
    last_guess: Option<(usize, usize)>,
    /// Current hits (accumulated from guess results).
    current_hits: BB,
}

impl AiPlayer {
    /// Create a new AI player with Hard difficulty (default).
    pub fn new() -> Self {
        Self::with_difficulty(Difficulty::Hard)
    }

    /// Create an AI player with the specified difficulty level.
    ///
    /// Difficulty levels control both temperature (decision quality) and
    /// hit_weight (targeting aggressiveness):
    /// - Easy: temperature=2.0, hit_weight=3.0 (weak targeting)
    /// - Medium: temperature=1.0, hit_weight=7.0 (balanced)
    /// - Hard: temperature=0.5, hit_weight=10.0 (strong, default)
    /// - Expert: temperature=0.2, hit_weight=13.0 (near-optimal)
    pub fn with_difficulty(difficulty: Difficulty) -> Self {
        let (temperature, hit_weight) = match difficulty {
            Difficulty::Easy => (2.0, 3.0),
            Difficulty::Medium => (1.0, 7.0),
            Difficulty::Hard => (0.5, 10.0),
            Difficulty::Expert => (0.2, 13.0),
        };
        Self {
            sunk: BB::new(),
            temperature,
            hit_weight,
            current_hits: BB::new(),
            last_guess: None,
        }
    }

    /// Create an AI player with custom parameters (for advanced users).
    ///
    /// # Parameters
    /// - `temperature`: Controls decision quality (0.1 = greedy, 2.0+ = random)
    /// - `hit_weight`: Bonus weight per hit (0 = no bonus, 10-20 typical)
    pub fn with_params(temperature: f64, hit_weight: f64) -> Self {
        Self {
            sunk: BB::new(),
            temperature,
            hit_weight,
            current_hits: BB::new(),
            last_guess: None,
        }
    }

    /// Set the temperature (builder-style).
    ///
    /// Temperature controls how greedily the AI exploits the probability distribution.
    /// Lower values (0.1-0.3) = more greedy, higher values (2.0+) = more random.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the hit weight (builder-style).
    ///
    /// Hit weight controls how aggressively the AI targets unsunk hits.
    /// Higher values = more aggressive targeting (typical range: 5-20).
    pub fn with_hit_weight(mut self, hit_weight: f64) -> Self {
        self.hit_weight = hit_weight;
        self
    }

    /// Identify and mark cells of a sunk ship.
    ///
    /// When a ship is sunk, we need to determine which hit cells belong to it.
    /// We search from the last guess coordinate to find all contiguous hits
    /// that form a line matching the ship's length.
    fn mark_ship_as_sunk(&mut self, ship_name: &str, last_coord: (usize, usize)) {
        // Find the ship definition to get its length
        let ship_len = SHIPS
            .iter()
            .find(|s| s.name() == ship_name)
            .map(|s| s.length())
            .unwrap_or(0);

        if ship_len == 0 {
            return;
        }

        let (r, c) = last_coord;

        // Try to find the ship extent in both horizontal and vertical directions
        let horizontal = self.find_ship_extent(r, c, ship_len, true);
        let vertical = self.find_ship_extent(r, c, ship_len, false);

        // Use whichever direction matches the ship length
        let cells = if horizontal.len() == ship_len {
            horizontal
        } else if vertical.len() == ship_len {
            vertical
        } else {
            // Fallback: just mark the last hit cell
            vec![(r, c)]
        };

        // Mark all identified cells as sunk
        for (row, col) in cells {
            let _ = self.sunk.set(row, col);
        }
    }

    /// Find the extent of a ship starting from a coordinate.
    /// Returns a vector of all contiguous hit cells in the specified direction.
    fn find_ship_extent(
        &self,
        start_r: usize,
        start_c: usize,
        max_len: usize,
        horizontal: bool,
    ) -> Vec<(usize, usize)> {
        let mut cells = vec![(start_r, start_c)];

        if horizontal {
            // Search backward
            for i in 1..max_len {
                if start_c >= i {
                    let c = start_c - i;
                    if self.current_hits.get(start_r, c).unwrap_or(false)
                        && !self.sunk.get(start_r, c).unwrap_or(false)
                    {
                        cells.insert(0, (start_r, c));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Search forward
            for i in 1..max_len {
                let c = start_c + i;
                if c < BOARD_SIZE as usize {
                    if self.current_hits.get(start_r, c).unwrap_or(false)
                        && !self.sunk.get(start_r, c).unwrap_or(false)
                    {
                        cells.push((start_r, c));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        } else {
            // Vertical: search backward
            for i in 1..max_len {
                if start_r >= i {
                    let r = start_r - i;
                    if self.current_hits.get(r, start_c).unwrap_or(false)
                        && !self.sunk.get(r, start_c).unwrap_or(false)
                    {
                        cells.insert(0, (r, start_c));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Search forward
            for i in 1..max_len {
                let r = start_r + i;
                if r < BOARD_SIZE as usize {
                    if self.current_hits.get(r, start_c).unwrap_or(false)
                        && !self.sunk.get(r, start_c).unwrap_or(false)
                    {
                        cells.push((r, start_c));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        cells
    }
}

impl Default for AiPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Player for AiPlayer {
    fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError> {
        for i in 0..NUM_SHIPS as usize {
            let (r, c, o) = board.random_placement(rng, i)?;
            board.place(i, r, c, o)?;
        }
        Ok(())
    }

    fn select_target(
        &mut self,
        rng: &mut SmallRng,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
    ) -> (usize, usize) {
        // Update current_hits for sunk ship tracking
        self.current_hits = *hits;

        // Compute probability distribution and select target
        let guess = ai::calc_pdf_and_guess(
            hits,
            misses,
            &self.sunk,
            remaining,
            rng,
            self.temperature,
            self.hit_weight,
        );

        self.last_guess = Some(guess);
        guess
    }

    fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult) {
        match result {
            GuessResult::Hit => {
                let _ = self.current_hits.set(coord.0, coord.1);
            }
            GuessResult::Sink(name) => {
                let _ = self.current_hits.set(coord.0, coord.1);
                self.mark_ship_as_sunk(&name, coord);
            }
            GuessResult::Miss => {}
        }
    }

    fn handle_opponent_guess(&mut self, _coord: (usize, usize), _result: GuessResult) {}
}
