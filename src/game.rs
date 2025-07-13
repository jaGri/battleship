use crate::{
    bitboard::BitBoard,
    board::{Board, BoardState},
    common::{BoardError, GuessResult},
    config::{BOARD_SIZE, NUM_SHIPS, SHIPS, TOTAL_SHIP_CELLS},
};

/// Bitboard type used for game state tracking.
type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Public state of the player's guesses against the opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuessBoardState {
    pub hits: BB,
    pub misses: BB,
}

/// Serializable overall game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameState {
    pub my_board: BoardState,
    pub my_guesses: GuessBoardState,
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
    enemy_ships_remaining: [bool; NUM_SHIPS as usize],
}

impl GameEngine {
    /// Create a new engine with an empty board and no guesses recorded.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            guess_hits: BB::new(),
            guess_misses: BB::new(),
            enemy_remaining: TOTAL_SHIP_CELLS,
            enemy_ships_remaining: [true; NUM_SHIPS as usize],
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
                    self.enemy_ships_remaining[idx] = false;
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

    /// Generate a serializable snapshot of the current state.
    pub fn state(&self) -> GameState {
        GameState {
            my_board: BoardState::from(&self.board),
            my_guesses: GuessBoardState {
                hits: self.guess_hits,
                misses: self.guess_misses,
            },
        }
    }

    /// Restore an engine from a previously saved state.
    pub fn from_state(state: GameState) -> Self {
        let enemy_remaining = TOTAL_SHIP_CELLS - state.my_guesses.hits.count_ones();
        Self {
            board: Board::from(state.my_board),
            guess_hits: state.my_guesses.hits,
            guess_misses: state.my_guesses.misses,
            enemy_remaining,
            enemy_ships_remaining: [true; NUM_SHIPS as usize],
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
pub fn enemy_ship_lengths_remaining(&self) -> [usize; NUM_SHIPS as usize] {
        let mut lens = [0usize; NUM_SHIPS as usize];
        for (i, def) in SHIPS.iter().enumerate() {
            if self.enemy_ships_remaining[i] {
                lens[i] = def.length();
            }
        }
        lens
    }
}

#[cfg_attr(feature = "std", async_trait::async_trait)]
impl crate::protocol::GameApi for GameEngine {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<crate::domain::GuessResult> {
        let res = self
            .opponent_guess(x as usize, y as usize)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(crate::domain::GuessResult::from(res))
    }

    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<crate::domain::Ship> {
        let states = self.board.ship_states();
        if ship_id >= states.len() {
            return Err(anyhow::anyhow!(BoardError::InvalidIndex));
        }
        Ok(crate::domain::Ship::from(states[ship_id]))
    }

    async fn sync_state(&mut self, _payload: crate::domain::SyncPayload) -> anyhow::Result<()> {
        // Protocol payload is placeholder; simply sync using current state helpers
        // when payloads carry real state in the future.
        Ok(())
    }

    fn status(&self) -> crate::domain::GameStatus {
        match GameEngine::status(self) {
            GameStatus::InProgress => crate::domain::GameStatus::InProgress,
            GameStatus::Won => crate::domain::GameStatus::Won,
            GameStatus::Lost => crate::domain::GameStatus::Lost,
        }
    }
}
