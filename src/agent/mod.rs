//! Player decision-making agents.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
#[cfg(feature = "std")]
use std::vec::Vec;

use rand::rngs::SmallRng;

use crate::engine::{
    ai, BitBoard, Board, BoardError, GuessBoard, GuessResult, Orientation, BOARD_SIZE, NUM_SHIPS,
    SHIPS,
};
use crate::input::UiEvent;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Coordinate type for board positions.
pub type Coordinate = (usize, usize);

/// Difficulty level for AI agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "std",
    derive(clap::ValueEnum, serde::Serialize, serde::Deserialize)
)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Backward-friendly difficulty name while the CLI is migrated.
pub type Difficulty = AiDifficulty;

/// A concrete ship placement selected by an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipPlacement {
    pub ship_index: usize,
    pub row: usize,
    pub col: usize,
    pub orientation: Orientation,
}

/// Events agents may observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    GuessResult {
        coord: Coordinate,
        result: GuessResult,
        by_local_player: bool,
    },
    GameOver {
        local_player_won: bool,
    },
}

/// Requests made by the app when it needs a player decision.
pub enum AgentRequest<'a> {
    PlaceShips {
        board: &'a Board,
    },
    SelectTarget {
        guess_board: &'a GuessBoard,
        remaining_ships: &'a [usize],
    },
    Observe(GameEvent),
}

/// Action returned by an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAction {
    None,
    PlaceShips(Vec<ShipPlacement>),
    Fire(Coordinate),
    Ready,
}

/// Decision-making role for human, AI, remote, and scripted players.
pub trait PlayerAgent {
    type Error;

    fn handle_request(
        &mut self,
        request: AgentRequest<'_>,
        rng: &mut SmallRng,
    ) -> Result<AgentAction, Self::Error>;
}

/// AI-backed player agent using the tracked probability and sunk-ship logic.
pub struct AiAgent {
    difficulty: AiDifficulty,
    sunk: BB,
    temperature: f64,
    hit_weight: f64,
    current_hits: BB,
    last_guess: Option<Coordinate>,
}

impl AiAgent {
    pub fn new(difficulty: AiDifficulty) -> Self {
        let (temperature, hit_weight) = match difficulty {
            AiDifficulty::Easy => (2.0, 3.0),
            AiDifficulty::Medium => (1.0, 7.0),
            AiDifficulty::Hard => (0.5, 10.0),
            AiDifficulty::Expert => (0.2, 13.0),
        };
        Self {
            difficulty,
            sunk: BB::new(),
            temperature,
            hit_weight,
            current_hits: BB::new(),
            last_guess: None,
        }
    }

    pub fn with_params(temperature: f64, hit_weight: f64) -> Self {
        Self {
            difficulty: AiDifficulty::Hard,
            sunk: BB::new(),
            temperature,
            hit_weight,
            current_hits: BB::new(),
            last_guess: None,
        }
    }

    pub fn difficulty(&self) -> AiDifficulty {
        self.difficulty
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_hit_weight(mut self, hit_weight: f64) -> Self {
        self.hit_weight = hit_weight;
        self
    }

    pub fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError> {
        for i in 0..NUM_SHIPS {
            let (r, c, o) = board.random_placement(rng, i)?;
            board.place(i, r, c, o)?;
        }
        Ok(())
    }

    pub fn select_target(
        &mut self,
        rng: &mut SmallRng,
        guess_board: &GuessBoard,
        remaining: &[usize],
    ) -> Coordinate {
        self.current_hits = guess_board.hits;

        let mut lengths = [0usize; NUM_SHIPS];
        for (idx, &length) in remaining.iter().enumerate().take(NUM_SHIPS) {
            lengths[idx] = length;
        }

        let guess = ai::calc_pdf_and_guess(
            &guess_board.hits,
            &guess_board.misses,
            &self.sunk,
            &lengths,
            rng,
            self.temperature,
            self.hit_weight,
        );
        let target = if guess_board.hits.get(guess.0, guess.1).unwrap_or(false)
            || guess_board.misses.get(guess.0, guess.1).unwrap_or(false)
        {
            first_unguessed(guess_board).unwrap_or(guess)
        } else {
            guess
        };
        self.last_guess = Some(target);
        target
    }

    fn handle_guess_result(&mut self, coord: Coordinate, result: GuessResult) {
        match result {
            GuessResult::Hit => {
                let _ = self.current_hits.set(coord.0, coord.1);
            }
            GuessResult::Sink(name) => {
                let _ = self.current_hits.set(coord.0, coord.1);
                self.mark_ship_as_sunk(name, coord);
            }
            GuessResult::Miss => {}
        }
    }

    fn mark_ship_as_sunk(&mut self, ship_name: &str, last_coord: Coordinate) {
        let ship_len = SHIPS
            .iter()
            .find(|ship| ship.name() == ship_name)
            .map(|ship| ship.length())
            .unwrap_or(0);
        if ship_len == 0 {
            return;
        }

        let (row, col) = last_coord;
        let horizontal = self.find_ship_extent(row, col, ship_len, true);
        let vertical = self.find_ship_extent(row, col, ship_len, false);
        let cells = if horizontal.len() == ship_len {
            horizontal
        } else if vertical.len() == ship_len {
            vertical
        } else {
            vec![last_coord]
        };

        for (r, c) in cells {
            let _ = self.sunk.set(r, c);
        }
    }

    fn find_ship_extent(
        &self,
        start_r: usize,
        start_c: usize,
        max_len: usize,
        horizontal: bool,
    ) -> Vec<Coordinate> {
        let mut cells = vec![(start_r, start_c)];

        for i in 1..max_len {
            let coord = if horizontal {
                start_c.checked_sub(i).map(|c| (start_r, c))
            } else {
                start_r.checked_sub(i).map(|r| (r, start_c))
            };
            if let Some((r, c)) = coord {
                if self.current_hits.get(r, c).unwrap_or(false)
                    && !self.sunk.get(r, c).unwrap_or(false)
                {
                    cells.insert(0, (r, c));
                    continue;
                }
            }
            break;
        }

        for i in 1..max_len {
            let coord = if horizontal {
                Some((start_r, start_c + i))
            } else {
                Some((start_r + i, start_c))
            };
            if let Some((r, c)) = coord {
                if r < BOARD_SIZE as usize
                    && c < BOARD_SIZE as usize
                    && self.current_hits.get(r, c).unwrap_or(false)
                    && !self.sunk.get(r, c).unwrap_or(false)
                {
                    cells.push((r, c));
                    continue;
                }
            }
            break;
        }

        cells
    }
}

fn first_unguessed(guess_board: &GuessBoard) -> Option<Coordinate> {
    for row in 0..BOARD_SIZE as usize {
        for col in 0..BOARD_SIZE as usize {
            if !guess_board.hits.get(row, col).unwrap_or(false)
                && !guess_board.misses.get(row, col).unwrap_or(false)
            {
                return Some((row, col));
            }
        }
    }
    None
}

impl Default for AiAgent {
    fn default() -> Self {
        Self::new(AiDifficulty::Hard)
    }
}

impl PlayerAgent for AiAgent {
    type Error = BoardError;

    fn handle_request(
        &mut self,
        request: AgentRequest<'_>,
        rng: &mut SmallRng,
    ) -> Result<AgentAction, Self::Error> {
        match request {
            AgentRequest::PlaceShips { board } => {
                let mut probe = board.clone();
                self.place_ships(rng, &mut probe)?;
                let placements = probe
                    .ship_states()
                    .iter()
                    .enumerate()
                    .filter_map(|(ship_index, state)| {
                        state.position.map(|(row, col, orientation)| ShipPlacement {
                            ship_index,
                            row,
                            col,
                            orientation,
                        })
                    })
                    .collect();
                Ok(AgentAction::PlaceShips(placements))
            }
            AgentRequest::SelectTarget {
                guess_board,
                remaining_ships,
            } => Ok(AgentAction::Fire(self.select_target(
                rng,
                guess_board,
                remaining_ships,
            ))),
            AgentRequest::Observe(GameEvent::GuessResult {
                coord,
                result,
                by_local_player: true,
            }) => {
                self.handle_guess_result(coord, result);
                Ok(AgentAction::None)
            }
            AgentRequest::Observe(_) => Ok(AgentAction::None),
        }
    }
}

/// Human agent that consumes already-normalized UI events.
#[derive(Default)]
pub struct HumanAgent {
    pending_target: Option<Coordinate>,
    pending_placements: Vec<ShipPlacement>,
    random_placement_requested: bool,
}

impl HumanAgent {
    pub fn on_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Target(coord) => {
                self.pending_target = Some(coord);
            }
            UiEvent::RandomPlacement => {
                self.pending_placements.clear();
                self.random_placement_requested = true;
            }
            UiEvent::ClearPlacements => {
                self.pending_placements.clear();
                self.random_placement_requested = false;
            }
            UiEvent::PlaceShip {
                ship_index,
                row,
                col,
                orientation,
            } => {
                self.random_placement_requested = false;
                self.pending_placements.push(ShipPlacement {
                    ship_index,
                    row,
                    col,
                    orientation,
                });
            }
            _ => {}
        }
    }
}

impl PlayerAgent for HumanAgent {
    type Error = BoardError;

    fn handle_request(
        &mut self,
        request: AgentRequest<'_>,
        rng: &mut SmallRng,
    ) -> Result<AgentAction, Self::Error> {
        match request {
            AgentRequest::PlaceShips { board } => {
                if self.pending_placements.len() == NUM_SHIPS && !self.random_placement_requested {
                    let placements = core::mem::take(&mut self.pending_placements);
                    return Ok(AgentAction::PlaceShips(placements));
                }

                let mut probe = board.clone();
                for ship_index in 0..NUM_SHIPS {
                    let (row, col, orientation) = probe.random_placement(rng, ship_index)?;
                    probe.place(ship_index, row, col, orientation)?;
                }
                self.random_placement_requested = false;
                let placements = probe
                    .ship_states()
                    .iter()
                    .enumerate()
                    .filter_map(|(ship_index, state)| {
                        state.position.map(|(row, col, orientation)| ShipPlacement {
                            ship_index,
                            row,
                            col,
                            orientation,
                        })
                    })
                    .collect();
                Ok(AgentAction::PlaceShips(placements))
            }
            AgentRequest::SelectTarget { .. } => Ok(self
                .pending_target
                .take()
                .map(AgentAction::Fire)
                .unwrap_or(AgentAction::None)),
            AgentRequest::Observe(_) => Ok(AgentAction::None),
        }
    }
}

/// Remote player agent. Transport-specific validation lives in protocol/app code.
#[derive(Default)]
pub struct RemoteAgent {
    pending_action: Option<AgentAction>,
}

impl RemoteAgent {
    pub fn push_action(&mut self, action: AgentAction) {
        self.pending_action = Some(action);
    }
}

impl PlayerAgent for RemoteAgent {
    type Error = core::convert::Infallible;

    fn handle_request(
        &mut self,
        _request: AgentRequest<'_>,
        _rng: &mut SmallRng,
    ) -> Result<AgentAction, Self::Error> {
        Ok(self.pending_action.take().unwrap_or(AgentAction::None))
    }
}

/// Deterministic test agent.
#[derive(Default)]
pub struct ScriptedAgent {
    actions: Vec<AgentAction>,
}

impl ScriptedAgent {
    pub fn new(actions: Vec<AgentAction>) -> Self {
        Self { actions }
    }
}

impl PlayerAgent for ScriptedAgent {
    type Error = core::convert::Infallible;

    fn handle_request(
        &mut self,
        _request: AgentRequest<'_>,
        _rng: &mut SmallRng,
    ) -> Result<AgentAction, Self::Error> {
        if self.actions.is_empty() {
            Ok(AgentAction::None)
        } else {
            Ok(self.actions.remove(0))
        }
    }
}
