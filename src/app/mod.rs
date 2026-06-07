//! Application state machine and game orchestration.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
#[cfg(feature = "std")]
use std::vec::Vec;

use rand::rngs::SmallRng;

use crate::agent::{AgentAction, AgentRequest, GameEvent, PlayerAgent, ShipPlacement};
use crate::engine::{BoardError, GameEngine, GameState, GameStatus, GuessBoard};
use crate::input::UiEvent;
use crate::protocol::WireMessage;
use crate::render::{GameEventView, GameView, ScreenView};

/// Snapshot saved by app runners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SavedGame {
    pub local_engine: GameState,
    pub opponent_engine: Option<GameState>,
    pub local_turn: bool,
    pub turn_number: u32,
}

/// Product-level app states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Title,
    MainMenu,
    SoloSetup,
    Pairing,
    Playing,
    ConnectionOverlay,
    GameOver,
}

/// Events accepted by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Ui(UiEvent),
    Agent(AgentAction),
    Transport(WireMessage),
    Tick,
    Loaded(Option<SavedGame>),
}

/// Commands emitted by the app runner.
pub enum AppCommand<'a> {
    Render,
    Send(WireMessage),
    Save(SavedGame),
    ClearSave,
    RequestAgent(AgentRequest<'a>),
}

/// Local match state. Remote games can leave `opponent_engine` empty.
pub struct MatchState {
    pub local_engine: GameEngine,
    pub opponent_engine: Option<GameEngine>,
    pub local_turn: bool,
    pub turn_number: u32,
    pub last_event: Option<GameEventView>,
}

impl MatchState {
    pub fn local_ai_game() -> Self {
        Self {
            local_engine: GameEngine::new(),
            opponent_engine: Some(GameEngine::new()),
            local_turn: true,
            turn_number: 0,
            last_event: None,
        }
    }

    pub fn saved_game(&self) -> SavedGame {
        SavedGame {
            local_engine: self.local_engine.state(),
            opponent_engine: self.opponent_engine.as_ref().map(GameEngine::state),
            local_turn: self.local_turn,
            turn_number: self.turn_number,
        }
    }

    pub fn from_saved_game(saved: SavedGame) -> Self {
        Self {
            local_engine: GameEngine::from_state(saved.local_engine),
            opponent_engine: saved.opponent_engine.map(GameEngine::from_state),
            local_turn: saved.local_turn,
            turn_number: saved.turn_number,
            last_event: None,
        }
    }
}

/// App coordinator for one local player.
pub struct BattleshipApp<A, O> {
    pub state: AppState,
    pub match_state: MatchState,
    pub local_agent: A,
    pub opponent_agent: O,
}

impl<A, O> BattleshipApp<A, O> {
    pub fn new_local_ai(local_agent: A, opponent_agent: O) -> Self {
        Self {
            state: AppState::SoloSetup,
            match_state: MatchState::local_ai_game(),
            local_agent,
            opponent_agent,
        }
    }

    pub fn view(&self) -> ScreenView<'_> {
        ScreenView::Game(GameView {
            my_board: self.match_state.local_engine.board(),
            guess_board: GuessBoard::from_engine(&self.match_state.local_engine),
            my_turn: self.match_state.local_turn,
            turn_number: self.match_state.turn_number,
            status: self.match_state.local_engine.status(),
            last_event: self.match_state.last_event,
        })
    }

    pub fn update(&mut self, event: AppEvent) -> Vec<AppCommand<'_>> {
        match event {
            AppEvent::Loaded(Some(saved)) => {
                self.match_state = MatchState::from_saved_game(saved);
                self.state = AppState::Playing;
                vec![AppCommand::Render]
            }
            AppEvent::Loaded(None) => vec![AppCommand::ClearSave, AppCommand::Render],
            AppEvent::Transport(msg) => vec![AppCommand::Send(msg), AppCommand::Render],
            AppEvent::Ui(_) | AppEvent::Agent(_) | AppEvent::Tick => vec![AppCommand::Render],
        }
    }
}

impl<A, O> BattleshipApp<A, O>
where
    A: PlayerAgent,
    O: PlayerAgent,
{
    pub fn place_ships(
        &mut self,
        local_rng: &mut SmallRng,
        opponent_rng: &mut SmallRng,
    ) -> Result<Vec<AppCommand<'_>>, BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        let local_action = self
            .local_agent
            .handle_request(
                AgentRequest::PlaceShips {
                    board: self.match_state.local_engine.board(),
                },
                local_rng,
            )
            .map_err(|_| BoardError::UnableToPlaceShip)?;
        self.apply_ship_action(true, local_action)?;

        let opponent_action = self
            .opponent_agent
            .handle_request(
                AgentRequest::PlaceShips {
                    board: self
                        .match_state
                        .opponent_engine
                        .as_ref()
                        .ok_or(BoardError::UnableToPlaceShip)?
                        .board(),
                },
                opponent_rng,
            )
            .map_err(|_| BoardError::UnableToPlaceShip)?;
        self.apply_ship_action(false, opponent_action)?;

        self.state = AppState::Playing;
        Ok(vec![
            AppCommand::Save(self.match_state.saved_game()),
            AppCommand::Render,
        ])
    }

    pub fn play_next_turn(
        &mut self,
        local_rng: &mut SmallRng,
        opponent_rng: &mut SmallRng,
    ) -> Result<Vec<AppCommand<'_>>, BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        if self.state != AppState::Playing {
            return Ok(vec![AppCommand::Render]);
        }

        self.match_state.turn_number += 1;
        if self.match_state.local_turn {
            self.execute_local_turn(local_rng)?;
        } else {
            self.execute_opponent_turn(opponent_rng)?;
        }

        if matches!(
            self.match_state.local_engine.status(),
            GameStatus::Won | GameStatus::Lost
        ) {
            self.state = AppState::GameOver;
            self.match_state.last_event = Some(GameEventView::GameOver {
                local_player_won: self.match_state.local_engine.status() == GameStatus::Won,
            });
        }

        Ok(vec![
            AppCommand::Save(self.match_state.saved_game()),
            AppCommand::Render,
        ])
    }

    fn execute_local_turn(&mut self, rng: &mut SmallRng) -> Result<(), BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        let guess_board = GuessBoard::from_engine(&self.match_state.local_engine);
        let remaining = self.match_state.local_engine.enemy_ship_lengths_remaining();
        let action = self
            .local_agent
            .handle_request(
                AgentRequest::SelectTarget {
                    guess_board: &guess_board,
                    remaining_ships: &remaining,
                },
                rng,
            )
            .map_err(|_| BoardError::UnableToPlaceShip)?;
        let coord = match action {
            AgentAction::Fire(coord) => coord,
            _ => return Err(BoardError::UnableToPlaceShip),
        };

        let opponent = self
            .match_state
            .opponent_engine
            .as_mut()
            .ok_or(BoardError::UnableToPlaceShip)?;
        let result = opponent.opponent_guess(coord.0, coord.1)?;
        self.match_state
            .local_engine
            .record_guess(coord.0, coord.1, result)?;
        self.finish_turn(coord, result, true, rng);
        Ok(())
    }

    fn execute_opponent_turn(&mut self, rng: &mut SmallRng) -> Result<(), BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        let opponent_engine = self
            .match_state
            .opponent_engine
            .as_ref()
            .ok_or(BoardError::UnableToPlaceShip)?;
        let guess_board = GuessBoard::from_engine(opponent_engine);
        let remaining = opponent_engine.enemy_ship_lengths_remaining();
        let action = self
            .opponent_agent
            .handle_request(
                AgentRequest::SelectTarget {
                    guess_board: &guess_board,
                    remaining_ships: &remaining,
                },
                rng,
            )
            .map_err(|_| BoardError::UnableToPlaceShip)?;
        let coord = match action {
            AgentAction::Fire(coord) => coord,
            _ => return Err(BoardError::UnableToPlaceShip),
        };

        let result = self
            .match_state
            .local_engine
            .opponent_guess(coord.0, coord.1)?;
        let opponent = self
            .match_state
            .opponent_engine
            .as_mut()
            .ok_or(BoardError::UnableToPlaceShip)?;
        opponent.record_guess(coord.0, coord.1, result)?;
        self.finish_turn(coord, result, false, rng);
        Ok(())
    }

    fn finish_turn(
        &mut self,
        coord: (usize, usize),
        result: crate::engine::GuessResult,
        by_local_player: bool,
        rng: &mut SmallRng,
    ) {
        self.match_state.last_event = Some(GameEventView::Guess {
            coord,
            result,
            by_local_player,
        });
        let event = GameEvent::GuessResult {
            coord,
            result,
            by_local_player,
        };
        let _ = self.local_agent.handle_request(AgentRequest::Observe(event), rng);
        let _ = self
            .opponent_agent
            .handle_request(AgentRequest::Observe(event), rng);
        self.match_state.local_turn = !self.match_state.local_turn;
    }

    fn apply_ship_action(&mut self, local: bool, action: AgentAction) -> Result<(), BoardError> {
        let placements = match action {
            AgentAction::PlaceShips(placements) => placements,
            _ => return Err(BoardError::UnableToPlaceShip),
        };

        let engine = if local {
            &mut self.match_state.local_engine
        } else {
            self.match_state
                .opponent_engine
                .as_mut()
                .ok_or(BoardError::UnableToPlaceShip)?
        };

        for ShipPlacement {
            ship_index,
            row,
            col,
            orientation,
        } in placements
        {
            engine.board_mut().place(ship_index, row, col, orientation)?;
        }
        Ok(())
    }
}
