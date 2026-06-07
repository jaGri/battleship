use super::{
    bitboard::BitBoard,
    board::{Board, BoardState},
    common::{BoardError, GuessResult},
    config::{BOARD_SIZE, NUM_SHIPS, SHIPS, TOTAL_SHIP_CELLS},
    ship::Ship,
};

/// Bitboard type used for game state tracking.
type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Public state of the player's guesses against the opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct GuessBoardState {
    pub hits: BB,
    pub misses: BB,
    pub ships: [Ship<u128, { BOARD_SIZE as usize }>; NUM_SHIPS],
}

/// Passive view of guesses made against an opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct GuessBoard {
    pub hits: BB,
    pub misses: BB,
    pub active_hits: BB,
}

impl GuessBoard {
    /// Create an empty guess board.
    pub fn new() -> Self {
        Self {
            hits: BB::new(),
            misses: BB::new(),
            active_hits: BB::new(),
        }
    }

    /// Build a guess board from a game engine.
    pub fn from_engine(engine: &GameEngine) -> Self {
        Self {
            hits: engine.guess_hits(),
            misses: engine.guess_misses(),
            active_hits: engine.active_hits(),
        }
    }

    /// Get a cell state: `Some(true)` for hit, `Some(false)` for miss, `None` for unknown.
    pub fn get_cell(&self, row: usize, col: usize) -> Option<bool> {
        if self.hits.get(row, col).unwrap_or(false) {
            Some(true)
        } else if self.misses.get(row, col).unwrap_or(false) {
            Some(false)
        } else {
            None
        }
    }
}

impl Default for GuessBoard {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable overall game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    any(feature = "std", feature = "ble"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct GameState {
    pub my_board: BoardState,
    pub my_guesses: GuessBoardState,
    /// Which enemy ships remain afloat (by index)
    pub enemy_ships_remaining: [bool; NUM_SHIPS],
    /// Number of enemy ship cells remaining
    pub enemy_remaining: usize,
}

/// Current status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

/// Core game logic holding the player's board and guess history.
pub struct GameEngine {
    board: Board,
    guess_hits: BB,
    guess_misses: BB,
    enemy_remaining: usize,
    enemy_ships: [Ship<u128, { BOARD_SIZE as usize }>; NUM_SHIPS],
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GameEngine {
    /// Create a new engine with an empty board and no guesses recorded.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            guess_hits: BB::new(),
            guess_misses: BB::new(),
            enemy_remaining: TOTAL_SHIP_CELLS,
            enemy_ships: initial_enemy_ships(),
        }
    }

    /// Mutable reference to the player's board for ship placement.
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    /// Immutable reference to the player's board.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Bitboard of our successful guesses on the opponent board.
    pub fn guess_hits(&self) -> BB {
        self.guess_hits
    }

    /// Bitboard of our missed guesses on the opponent board.
    pub fn guess_misses(&self) -> BB {
        self.guess_misses
    }

    /// Bitboard of hits that are still relevant for targeting.
    pub fn active_hits(&self) -> BB {
        self.guess_hits & !self.sunk_enemy_footprints()
    }

    /// Public state of enemy ships known to this engine.
    pub fn enemy_ships(&self) -> [Ship<u128, { BOARD_SIZE as usize }>; NUM_SHIPS] {
        self.enemy_ships
    }

    /// Handle an opponent guess on the player's board.
    pub fn opponent_guess(&mut self, row: usize, col: usize) -> Result<GuessResult, BoardError> {
        self.board.guess(row, col)
    }

    /// Record the result of a guess made against the opponent.
    pub fn record_guess(
        &mut self,
        row: usize,
        col: usize,
        result: GuessResult,
    ) -> Result<(), BoardError> {
        if self.guess_hits.get(row, col)? || self.guess_misses.get(row, col)? {
            return Err(BoardError::AlreadyGuessed);
        }
        match result {
            GuessResult::Hit => {
                self.guess_hits.set(row, col)?;
                self.enemy_remaining = self.enemy_remaining.saturating_sub(1);
            }
            GuessResult::Sink(name) => {
                self.guess_hits.set(row, col)?;
                self.enemy_remaining = self.enemy_remaining.saturating_sub(1);
                if let Some(idx) = SHIPS.iter().position(|s| s.name() == name) {
                    self.enemy_ships[idx].sunk = true;
                } else {
                    return Err(BoardError::NameNotFound);
                }
            }
            GuessResult::Miss => {
                self.guess_misses.set(row, col)?;
            }
        }
        Ok(())
    }

    /// Record a sink result with the fixed-size public footprint revealed by
    /// the opponent.
    pub fn record_sink_with_footprint(
        &mut self,
        row: usize,
        col: usize,
        ship_name: &'static str,
        footprint: BB,
    ) -> Result<(), BoardError> {
        if self.guess_hits.get(row, col)? || self.guess_misses.get(row, col)? {
            return Err(BoardError::AlreadyGuessed);
        }
        let idx = SHIPS
            .iter()
            .position(|s| s.name() == ship_name)
            .ok_or(BoardError::NameNotFound)?;
        if footprint.count_ones() != SHIPS[idx].length() || !footprint.get(row, col)? {
            return Err(BoardError::InvalidSunkShipFootprint);
        }
        if !(footprint & self.guess_misses).is_empty() {
            return Err(BoardError::InvalidSunkShipFootprint);
        }

        let mut public_hits = self.guess_hits;
        public_hits.set(row, col)?;
        if !(footprint & !public_hits).is_empty() {
            return Err(BoardError::InvalidSunkShipFootprint);
        }

        self.guess_hits.set(row, col)?;
        self.enemy_remaining = self.enemy_remaining.saturating_sub(1);
        self.enemy_ships[idx].reveal_sunk(footprint);
        Ok(())
    }

    /// Generate a serializable snapshot of the current state.
    pub fn state(&self) -> GameState {
        GameState {
            my_board: BoardState::from(&self.board),
            my_guesses: GuessBoardState {
                hits: self.guess_hits,
                misses: self.guess_misses,
                ships: self.enemy_ships,
            },
            enemy_ships_remaining: self.enemy_ships_remaining(),
            enemy_remaining: self.enemy_remaining,
        }
    }

    /// Restore an engine from a previously saved state.
    pub fn from_state(state: GameState) -> Self {
        Self {
            board: Board::from(state.my_board),
            guess_hits: state.my_guesses.hits,
            guess_misses: state.my_guesses.misses,
            enemy_remaining: state.enemy_remaining,
            enemy_ships: core::array::from_fn(|i| {
                let mut ship = state.my_guesses.ships[i].with_definition(SHIPS[i]);
                ship.sunk = ship.sunk || !state.enemy_ships_remaining[i];
                ship
            }),
        }
    }

    /// Evaluate the current game status.
    pub fn status(&self) -> GameStatus {
        if self.board.all_sunk() {
            GameStatus::Lost
        } else if self.enemy_remaining == 0 {
            GameStatus::Won
        } else {
            GameStatus::InProgress
        }
    }

    /// Lengths of enemy ships that have not yet been sunk. Entries are zero
    /// for ships already sunk, maintaining fixed-size output for `no_std`
    /// callers.
    pub fn enemy_ship_lengths_remaining(&self) -> [usize; NUM_SHIPS] {
        let mut lens = [0usize; NUM_SHIPS];
        for (i, ship) in self.enemy_ships.iter().enumerate() {
            let _ = SHIPS[i];
            lens[i] = ship.remaining_length();
        }
        lens
    }

    fn enemy_ships_remaining(&self) -> [bool; NUM_SHIPS] {
        core::array::from_fn(|i| !self.enemy_ships[i].is_sunk())
    }

    fn sunk_enemy_footprints(&self) -> BB {
        let mut footprints = BB::new();
        for ship in self.enemy_ships.iter() {
            if ship.is_sunk() {
                footprints |= ship.mask();
            }
        }
        footprints
    }
}

fn initial_enemy_ships() -> [Ship<u128, { BOARD_SIZE as usize }>; NUM_SHIPS] {
    core::array::from_fn(|i| Ship::unknown(SHIPS[i]))
}
