//! Application state machine and game orchestration.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
#[cfg(feature = "std")]
use std::vec::Vec;

use rand::rngs::SmallRng;

use crate::agent::{
    AgentAction, AgentRequest, AiDifficulty, GameEvent, PlayerAgent, ShipPlacement,
};
use crate::engine::{
    BoardError, GameEngine, GameState, GameStatus, GuessBoard, GuessBoardState, GuessResult, SHIPS,
};
use crate::input::UiEvent;
use crate::protocol::domain::{RemotePlayer, RemoteSyncPayload};
use crate::protocol::{WireMessage, PROTOCOL_VERSION};
use crate::render::{ConnectionView, GameEventView, GameView, MenuView, MessageView, ScreenView};

const MAIN_MENU_ITEMS: [&str; 6] = [
    "New Solo Game",
    "Resume Game",
    "Remote Host",
    "Remote Join",
    "Difficulty",
    "Quit",
];
const SOLO_SETUP_ITEMS: [&str; 3] = ["Random placement", "Manual placement", "Back"];
const DIFFICULTY_ITEMS: [&str; 5] = ["Easy", "Medium", "Hard", "Expert", "Back"];

/// Ship setup mode requested from an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementMode {
    Prompt,
    Random,
    Manual,
}

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
    DifficultyMenu,
    Pairing,
    Playing,
    ConnectionOverlay,
    GameOver,
}

/// Player side addressed by reducer events and commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSide {
    Local,
    Opponent,
}

/// Role used by a remote session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteRole {
    Host,
    Guest,
}

/// User-visible connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    VersionMismatch,
    OutOfSync,
}

impl ConnectionStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::VersionMismatch => "protocol version mismatch",
            Self::OutOfSync => "out of sync",
        }
    }
}

/// Metadata for a connected remote match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSession {
    pub role: RemoteRole,
    pub status: ConnectionStatus,
    pub next_seq: u64,
    pub last_received_seq: Option<u64>,
    pub local_ready: bool,
    pub peer_ready: bool,
    pub awaiting_status_seq: Option<u64>,
    pub pending_target: Option<(usize, usize)>,
    pub cached_response: Option<WireMessage>,
}

impl RemoteSession {
    pub fn new(role: RemoteRole) -> Self {
        Self {
            role,
            status: ConnectionStatus::Connecting,
            next_seq: 1,
            last_received_seq: None,
            local_ready: false,
            peer_ready: false,
            awaiting_status_seq: None,
            pending_target: None,
            cached_response: None,
        }
    }

    fn connected(role: RemoteRole) -> Self {
        let mut session = Self::new(role);
        session.status = ConnectionStatus::Connected;
        session
    }

    fn next_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1).max(1);
        seq
    }
}

/// Current match mode.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum MatchMode {
    #[default]
    Solo,
    Remote(RemoteSession),
}

/// Agent work kind a runner may need to schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPromptKind {
    PlaceShips(PlacementMode),
    SelectTarget,
    Observe(GameEvent),
}

/// Agent work a runner may need to schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentPrompt {
    pub side: PlayerSide,
    pub kind: AgentPromptKind,
}

/// Events accepted by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum AppEvent {
    Ui(UiEvent),
    Agent {
        side: PlayerSide,
        action: AgentAction,
    },
    Transport(WireMessage),
    TransportConnected,
    TransportDisconnected,
    Tick,
    Loaded(Option<SavedGame>),
}

/// Commands emitted by the app runner.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum AppCommand {
    Render,
    Send(WireMessage),
    LoadActiveSave,
    Save(SavedGame),
    ClearSave,
    ConfigureDifficulty(AiDifficulty),
    RequestAgent(AgentPrompt),
    Exit,
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

    pub fn remote_game(role: RemoteRole) -> Self {
        Self {
            local_engine: GameEngine::new(),
            opponent_engine: None,
            local_turn: matches!(role, RemoteRole::Host),
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
    pub selected_menu_item: usize,
    pub match_mode: MatchMode,
    pub pending_prompt: Option<AgentPrompt>,
    pub last_notice: Option<&'static str>,
    pub ai_difficulty: AiDifficulty,
    local_ships_placed: bool,
    opponent_ships_placed: bool,
}

impl<A, O> BattleshipApp<A, O> {
    pub fn new_local_ai(local_agent: A, opponent_agent: O) -> Self {
        Self {
            state: AppState::Title,
            match_state: MatchState::local_ai_game(),
            local_agent,
            opponent_agent,
            selected_menu_item: 0,
            match_mode: MatchMode::Solo,
            pending_prompt: None,
            last_notice: None,
            ai_difficulty: AiDifficulty::Hard,
            local_ships_placed: false,
            opponent_ships_placed: false,
        }
    }

    pub fn view(&self) -> ScreenView<'_> {
        match self.state {
            AppState::Title => ScreenView::Title,
            AppState::MainMenu => ScreenView::Menu(MenuView {
                title: "Battleship",
                items: &MAIN_MENU_ITEMS,
                selected: self.selected_menu_item,
                notice: self.last_notice,
            }),
            AppState::SoloSetup => ScreenView::Menu(MenuView {
                title: "Solo Setup",
                items: &SOLO_SETUP_ITEMS,
                selected: self.selected_menu_item,
                notice: self.last_notice,
            }),
            AppState::DifficultyMenu => ScreenView::Menu(MenuView {
                title: "Difficulty",
                items: &DIFFICULTY_ITEMS,
                selected: self.selected_menu_item,
                notice: Some(difficulty_notice(self.ai_difficulty)),
            }),
            AppState::Pairing => ScreenView::Pairing(ConnectionView {
                code: None,
                connected: self.remote_status() == Some(ConnectionStatus::Connected),
                status: self
                    .remote_status()
                    .unwrap_or(ConnectionStatus::Connecting)
                    .label(),
            }),
            AppState::ConnectionOverlay => ScreenView::ConnectionOverlay(ConnectionView {
                code: None,
                connected: false,
                status: self
                    .remote_status()
                    .unwrap_or(ConnectionStatus::Disconnected)
                    .label(),
            }),
            AppState::Playing => self.game_view(),
            AppState::GameOver => ScreenView::Message(MessageView {
                title: "Game Over",
                body: match self.match_state.local_engine.status() {
                    GameStatus::Won => "You won.",
                    GameStatus::Lost => "You lost.",
                    GameStatus::InProgress => "The game ended before completion.",
                },
            }),
        }
    }

    fn game_view(&self) -> ScreenView<'_> {
        ScreenView::Game(GameView {
            my_board: self.match_state.local_engine.board(),
            guess_board: GuessBoard::from_engine(&self.match_state.local_engine),
            my_turn: self.match_state.local_turn,
            turn_number: self.match_state.turn_number,
            status: self.match_state.local_engine.status(),
            last_event: self.match_state.last_event,
        })
    }

    pub fn update(&mut self, event: AppEvent) -> Vec<AppCommand> {
        self.last_notice = None;
        match event {
            AppEvent::Loaded(Some(saved)) => {
                self.match_state = MatchState::from_saved_game(saved);
                self.state = AppState::Playing;
                self.local_ships_placed = true;
                self.opponent_ships_placed = self.match_state.opponent_engine.is_some();
                self.pending_prompt = None;
                self.save_request_and_render()
            }
            AppEvent::Loaded(None) => {
                self.state = AppState::MainMenu;
                self.selected_menu_item = 1;
                self.pending_prompt = None;
                self.last_notice = Some("No saved game.");
                vec![AppCommand::ClearSave, AppCommand::Render]
            }
            AppEvent::TransportConnected => self.handle_transport_connected(),
            AppEvent::TransportDisconnected => self.handle_transport_disconnected(),
            AppEvent::Ui(event) => self.handle_ui(event),
            AppEvent::Agent { side, action } => self.handle_agent(side, action),
            AppEvent::Transport(msg) => self.handle_transport(msg),
            AppEvent::Tick => self.handle_tick(),
        }
    }

    fn handle_ui(&mut self, event: UiEvent) -> Vec<AppCommand> {
        match self.state {
            AppState::Title => match event {
                UiEvent::Start | UiEvent::Confirm => {
                    self.state = AppState::MainMenu;
                    vec![AppCommand::Render]
                }
                UiEvent::Back => vec![AppCommand::Render, AppCommand::Exit],
                _ => vec![AppCommand::Render],
            },
            AppState::MainMenu => self.handle_menu_ui(event),
            AppState::SoloSetup => self.handle_solo_setup_ui(event),
            AppState::DifficultyMenu => self.handle_difficulty_ui(event),
            AppState::ConnectionOverlay => match event {
                UiEvent::Confirm | UiEvent::Start => self.request_remote_resume(),
                UiEvent::Back => {
                    self.state = AppState::MainMenu;
                    vec![AppCommand::Render]
                }
                _ => vec![AppCommand::Render],
            },
            AppState::GameOver => match event {
                UiEvent::Confirm | UiEvent::Start | UiEvent::Back => {
                    self.state = AppState::MainMenu;
                    self.pending_prompt = None;
                    vec![AppCommand::Render]
                }
                _ => vec![AppCommand::Render],
            },
            _ => vec![AppCommand::Render],
        }
    }

    fn handle_menu_ui(&mut self, event: UiEvent) -> Vec<AppCommand> {
        match event {
            UiEvent::Up => {
                self.selected_menu_item = if self.selected_menu_item == 0 {
                    MAIN_MENU_ITEMS.len() - 1
                } else {
                    self.selected_menu_item - 1
                };
                vec![AppCommand::Render]
            }
            UiEvent::Down => {
                self.selected_menu_item = (self.selected_menu_item + 1) % MAIN_MENU_ITEMS.len();
                vec![AppCommand::Render]
            }
            UiEvent::Confirm | UiEvent::Start => match self.selected_menu_item {
                0 => self.start_solo_setup(),
                1 => vec![AppCommand::LoadActiveSave],
                2 => self.start_remote_pairing(RemoteRole::Host),
                3 => self.start_remote_pairing(RemoteRole::Guest),
                4 => self.start_difficulty_menu(),
                _ => vec![AppCommand::Render, AppCommand::Exit],
            },
            UiEvent::Back => vec![AppCommand::Render, AppCommand::Exit],
            _ => vec![AppCommand::Render],
        }
    }

    fn handle_solo_setup_ui(&mut self, event: UiEvent) -> Vec<AppCommand> {
        match event {
            UiEvent::Up => {
                self.selected_menu_item = if self.selected_menu_item == 0 {
                    SOLO_SETUP_ITEMS.len() - 1
                } else {
                    self.selected_menu_item - 1
                };
                vec![AppCommand::Render]
            }
            UiEvent::Down => {
                self.selected_menu_item = (self.selected_menu_item + 1) % SOLO_SETUP_ITEMS.len();
                vec![AppCommand::Render]
            }
            UiEvent::Confirm | UiEvent::Start => match self.selected_menu_item {
                0 => self.request_place(PlayerSide::Local, PlacementMode::Random),
                1 => self.request_place(PlayerSide::Local, PlacementMode::Manual),
                _ => {
                    self.state = AppState::MainMenu;
                    self.selected_menu_item = 0;
                    vec![AppCommand::Render]
                }
            },
            UiEvent::Back => {
                self.state = AppState::MainMenu;
                self.selected_menu_item = 0;
                vec![AppCommand::Render]
            }
            _ => vec![AppCommand::Render],
        }
    }

    fn start_difficulty_menu(&mut self) -> Vec<AppCommand> {
        self.state = AppState::DifficultyMenu;
        self.selected_menu_item = difficulty_index(self.ai_difficulty);
        vec![AppCommand::Render]
    }

    fn handle_difficulty_ui(&mut self, event: UiEvent) -> Vec<AppCommand> {
        match event {
            UiEvent::Up => {
                self.selected_menu_item = if self.selected_menu_item == 0 {
                    DIFFICULTY_ITEMS.len() - 1
                } else {
                    self.selected_menu_item - 1
                };
                vec![AppCommand::Render]
            }
            UiEvent::Down => {
                self.selected_menu_item = (self.selected_menu_item + 1) % DIFFICULTY_ITEMS.len();
                vec![AppCommand::Render]
            }
            UiEvent::Confirm | UiEvent::Start => {
                if let Some(difficulty) = difficulty_from_index(self.selected_menu_item) {
                    self.ai_difficulty = difficulty;
                    self.state = AppState::MainMenu;
                    self.selected_menu_item = 4;
                    self.last_notice = Some("Difficulty updated.");
                    vec![
                        AppCommand::ConfigureDifficulty(difficulty),
                        AppCommand::Render,
                    ]
                } else {
                    self.state = AppState::MainMenu;
                    self.selected_menu_item = 4;
                    vec![AppCommand::Render]
                }
            }
            UiEvent::Back => {
                self.state = AppState::MainMenu;
                self.selected_menu_item = 4;
                vec![AppCommand::Render]
            }
            _ => vec![AppCommand::Render],
        }
    }

    fn handle_agent(&mut self, side: PlayerSide, action: AgentAction) -> Vec<AppCommand> {
        let Some(prompt) = self.pending_prompt else {
            self.last_notice = Some("No agent decision is pending.");
            return vec![AppCommand::Render];
        };

        if prompt.side != side {
            self.last_notice = Some("Agent decision came from the wrong side.");
            return vec![AppCommand::Render];
        }

        match (self.state, prompt.kind) {
            (AppState::SoloSetup, AgentPromptKind::PlaceShips(_)) => {
                self.handle_setup_placement(side, action)
            }
            (AppState::Pairing, AgentPromptKind::PlaceShips(_)) => {
                self.handle_remote_placement(side, action)
            }
            (AppState::Playing, AgentPromptKind::SelectTarget) => {
                self.handle_playing_agent(side, action)
            }
            _ => {
                self.last_notice = Some("Agent decision does not apply to the current state.");
                vec![AppCommand::Render]
            }
        }
    }

    fn handle_transport(&mut self, msg: WireMessage) -> Vec<AppCommand> {
        match msg {
            WireMessage::Handshake { version } => self.handle_handshake(version),
            WireMessage::HandshakeAck { version } => self.handle_handshake_ack(version),
            WireMessage::Ready { version, .. } if version == PROTOCOL_VERSION => {
                self.handle_remote_ready()
            }
            WireMessage::Guess { version, seq, x, y } if version == PROTOCOL_VERSION => {
                self.handle_remote_guess(seq, x as usize, y as usize)
            }
            WireMessage::StatusResp { version, seq, res } if version == PROTOCOL_VERSION => {
                self.handle_status_response(seq, res)
            }
            WireMessage::ResumeReq { version, seq } if version == PROTOCOL_VERSION => {
                let payload = self.remote_sync_payload();
                vec![
                    AppCommand::Send(WireMessage::ResumeAck {
                        version: PROTOCOL_VERSION,
                        seq,
                        payload,
                    }),
                    AppCommand::Render,
                ]
            }
            WireMessage::ResumeAck {
                version, payload, ..
            }
            | WireMessage::PrivateSync {
                version, payload, ..
            } if version == PROTOCOL_VERSION => self.apply_private_sync(payload),
            WireMessage::Heartbeat { version } if version == PROTOCOL_VERSION => {
                vec![AppCommand::Render]
            }
            _ => {
                self.mark_remote_status(ConnectionStatus::VersionMismatch);
                self.state = AppState::ConnectionOverlay;
                vec![AppCommand::Render]
            }
        }
    }

    fn handle_tick(&mut self) -> Vec<AppCommand> {
        match self.state {
            AppState::Playing if self.pending_prompt.is_none() => self.request_current_turn_agent(),
            _ => vec![AppCommand::Render],
        }
    }

    fn handle_transport_connected(&mut self) -> Vec<AppCommand> {
        self.mark_remote_status(ConnectionStatus::Connected);
        if self.state == AppState::ConnectionOverlay {
            self.state = AppState::Pairing;
        }
        vec![AppCommand::Render]
    }

    fn handle_transport_disconnected(&mut self) -> Vec<AppCommand> {
        self.mark_remote_status(ConnectionStatus::Disconnected);
        self.state = AppState::ConnectionOverlay;
        self.pending_prompt = None;
        vec![AppCommand::Render]
    }

    fn start_solo_setup(&mut self) -> Vec<AppCommand> {
        self.state = AppState::SoloSetup;
        self.match_mode = MatchMode::Solo;
        self.match_state = MatchState::local_ai_game();
        self.local_ships_placed = false;
        self.opponent_ships_placed = false;
        self.pending_prompt = None;
        self.selected_menu_item = 0;
        vec![AppCommand::Render]
    }

    fn start_remote_pairing(&mut self, role: RemoteRole) -> Vec<AppCommand> {
        self.state = AppState::Pairing;
        self.match_mode = MatchMode::Remote(RemoteSession::new(role));
        self.match_state = MatchState::remote_game(role);
        self.local_ships_placed = false;
        self.opponent_ships_placed = true;
        self.pending_prompt = None;

        let mut commands = Vec::new();
        if matches!(role, RemoteRole::Host) {
            commands.push(AppCommand::Send(WireMessage::Handshake {
                version: PROTOCOL_VERSION,
            }));
        }
        commands.push(AppCommand::Render);
        commands
    }

    fn request_place(&mut self, side: PlayerSide, mode: PlacementMode) -> Vec<AppCommand> {
        self.pending_prompt = Some(AgentPrompt {
            side,
            kind: AgentPromptKind::PlaceShips(mode),
        });
        vec![
            AppCommand::RequestAgent(AgentPrompt {
                side,
                kind: AgentPromptKind::PlaceShips(mode),
            }),
            AppCommand::Render,
        ]
    }

    fn request_current_turn_agent(&mut self) -> Vec<AppCommand> {
        if matches!(
            &self.match_mode,
            MatchMode::Remote(RemoteSession {
                awaiting_status_seq: Some(_),
                ..
            })
        ) {
            return vec![AppCommand::Render];
        }
        let side = if self.match_state.local_turn {
            PlayerSide::Local
        } else {
            PlayerSide::Opponent
        };
        self.pending_prompt = Some(AgentPrompt {
            side,
            kind: AgentPromptKind::SelectTarget,
        });
        vec![
            AppCommand::RequestAgent(AgentPrompt {
                side,
                kind: AgentPromptKind::SelectTarget,
            }),
            AppCommand::Render,
        ]
    }

    fn handle_setup_placement(&mut self, side: PlayerSide, action: AgentAction) -> Vec<AppCommand> {
        if self.apply_ship_action(side, action).is_err() {
            self.last_notice = Some("Unable to place ships.");
            return vec![AppCommand::Render];
        }

        self.pending_prompt = None;
        match side {
            PlayerSide::Local => {
                self.local_ships_placed = true;
                self.request_place(PlayerSide::Opponent, PlacementMode::Random)
            }
            PlayerSide::Opponent => {
                self.opponent_ships_placed = true;
                self.state = AppState::Playing;
                self.save_request_and_render()
            }
        }
    }

    fn handle_remote_placement(
        &mut self,
        side: PlayerSide,
        action: AgentAction,
    ) -> Vec<AppCommand> {
        if side != PlayerSide::Local {
            self.last_notice = Some("Only the local board is placed for remote games.");
            return vec![AppCommand::Render];
        }
        if self.apply_ship_action(side, action).is_err() {
            self.last_notice = Some("Unable to place ships.");
            return vec![AppCommand::Render];
        }

        self.pending_prompt = None;
        self.local_ships_placed = true;
        let seq = self.remote_next_seq();
        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.local_ready = true;
        }

        let mut commands = vec![AppCommand::Send(WireMessage::Ready {
            version: PROTOCOL_VERSION,
            seq,
        })];
        commands.extend(self.enter_remote_play_if_ready());
        commands
    }

    fn handle_playing_agent(&mut self, side: PlayerSide, action: AgentAction) -> Vec<AppCommand> {
        let AgentAction::Fire(coord) = action else {
            self.last_notice = Some("Expected a target selection.");
            return vec![AppCommand::Render];
        };
        self.pending_prompt = None;

        match self.match_mode {
            MatchMode::Solo => match self.apply_solo_fire(side, coord) {
                Ok(()) => self.after_turn_commands(),
                Err(_) => {
                    self.last_notice = Some("Unable to apply target selection.");
                    vec![AppCommand::Render]
                }
            },
            MatchMode::Remote(_) if side == PlayerSide::Local => self.send_remote_guess(coord),
            MatchMode::Remote(_) => {
                self.last_notice = Some("Remote opponent actions arrive through transport.");
                vec![AppCommand::Render]
            }
        }
    }

    fn handle_handshake(&mut self, version: u8) -> Vec<AppCommand> {
        if version != PROTOCOL_VERSION {
            self.mark_remote_status(ConnectionStatus::VersionMismatch);
            self.state = AppState::ConnectionOverlay;
            return vec![AppCommand::Render];
        }

        if !matches!(self.match_mode, MatchMode::Remote(_)) {
            self.match_mode = MatchMode::Remote(RemoteSession::connected(RemoteRole::Guest));
            self.match_state = MatchState::remote_game(RemoteRole::Guest);
            self.local_ships_placed = false;
            self.opponent_ships_placed = true;
        }
        self.state = AppState::Pairing;
        self.mark_remote_status(ConnectionStatus::Connected);
        let mut commands = vec![AppCommand::Send(WireMessage::HandshakeAck {
            version: PROTOCOL_VERSION,
        })];
        if !self.local_ships_placed {
            commands.extend(self.request_place(PlayerSide::Local, PlacementMode::Random));
        } else {
            commands.push(AppCommand::Render);
        }
        commands
    }

    fn handle_handshake_ack(&mut self, version: u8) -> Vec<AppCommand> {
        if version != PROTOCOL_VERSION {
            self.mark_remote_status(ConnectionStatus::VersionMismatch);
            self.state = AppState::ConnectionOverlay;
            return vec![AppCommand::Render];
        }
        self.mark_remote_status(ConnectionStatus::Connected);
        if self.state != AppState::Pairing {
            self.state = AppState::Pairing;
        }
        if !self.local_ships_placed {
            self.request_place(PlayerSide::Local, PlacementMode::Random)
        } else {
            vec![AppCommand::Render]
        }
    }

    fn handle_remote_ready(&mut self) -> Vec<AppCommand> {
        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.peer_ready = true;
        }
        self.enter_remote_play_if_ready()
    }

    fn enter_remote_play_if_ready(&mut self) -> Vec<AppCommand> {
        let ready = match &self.match_mode {
            MatchMode::Remote(session) => session.local_ready && session.peer_ready,
            MatchMode::Solo => false,
        };

        if ready {
            self.state = AppState::Playing;
            self.save_request_and_render()
        } else {
            vec![AppCommand::Render]
        }
    }

    fn send_remote_guess(&mut self, coord: (usize, usize)) -> Vec<AppCommand> {
        let seq = self.remote_next_seq();
        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.awaiting_status_seq = Some(seq);
            session.pending_target = Some(coord);
        }
        vec![
            AppCommand::Send(WireMessage::Guess {
                version: PROTOCOL_VERSION,
                seq,
                x: coord.0 as u8,
                y: coord.1 as u8,
            }),
            AppCommand::Render,
        ]
    }

    fn handle_remote_guess(&mut self, seq: u64, row: usize, col: usize) -> Vec<AppCommand> {
        if let MatchMode::Remote(session) = &mut self.match_mode {
            if session.last_received_seq == Some(seq) {
                if let Some(msg) = session.cached_response.clone() {
                    return vec![AppCommand::Send(msg), AppCommand::Render];
                }
            }

            if let Some(last) = session.last_received_seq {
                if seq != last.wrapping_add(1) {
                    session.status = ConnectionStatus::OutOfSync;
                    self.state = AppState::ConnectionOverlay;
                    let sync_seq = session.next_seq();
                    let payload = self.remote_sync_payload();
                    return vec![
                        AppCommand::Send(WireMessage::PrivateSync {
                            version: PROTOCOL_VERSION,
                            seq: sync_seq,
                            payload,
                        }),
                        AppCommand::Render,
                    ];
                }
            }
        }

        let result = match self.match_state.local_engine.opponent_guess(row, col) {
            Ok(result) => result,
            Err(_) => {
                self.mark_remote_status(ConnectionStatus::OutOfSync);
                self.state = AppState::ConnectionOverlay;
                return vec![AppCommand::Render];
            }
        };
        self.finish_turn((row, col), result, false);
        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.last_received_seq = Some(seq);
            let response = WireMessage::StatusResp {
                version: PROTOCOL_VERSION,
                seq,
                res: result.into(),
            };
            session.cached_response = Some(response.clone());
            let mut commands = vec![AppCommand::Send(response)];
            commands.extend(self.after_turn_commands());
            commands
        } else {
            vec![AppCommand::Render]
        }
    }

    fn handle_status_response(
        &mut self,
        seq: u64,
        res: crate::protocol::domain::GuessResult,
    ) -> Vec<AppCommand> {
        let expected = match &self.match_mode {
            MatchMode::Remote(session) => session.awaiting_status_seq,
            MatchMode::Solo => None,
        };
        if expected != Some(seq) {
            self.mark_remote_status(ConnectionStatus::OutOfSync);
            self.state = AppState::ConnectionOverlay;
            return vec![AppCommand::Render];
        }

        let result = match core_guess_result(res) {
            Ok(result) => result,
            Err(_) => {
                self.mark_remote_status(ConnectionStatus::OutOfSync);
                self.state = AppState::ConnectionOverlay;
                return vec![AppCommand::Render];
            }
        };
        let coord = match &self.match_mode {
            MatchMode::Remote(session) => session.pending_target,
            MatchMode::Solo => None,
        };
        if let Some(coord) = coord {
            if self
                .match_state
                .local_engine
                .record_guess(coord.0, coord.1, result)
                .is_err()
            {
                self.mark_remote_status(ConnectionStatus::OutOfSync);
                self.state = AppState::ConnectionOverlay;
                return vec![AppCommand::Render];
            }
            self.finish_turn(coord, result, true);
        } else {
            self.mark_remote_status(ConnectionStatus::OutOfSync);
            self.state = AppState::ConnectionOverlay;
            return vec![AppCommand::Render];
        }

        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.awaiting_status_seq = None;
            session.pending_target = None;
        }
        self.after_turn_commands()
    }

    fn request_remote_resume(&mut self) -> Vec<AppCommand> {
        let seq = self.remote_next_seq();
        self.mark_remote_status(ConnectionStatus::Reconnecting);
        vec![
            AppCommand::Send(WireMessage::ResumeReq {
                version: PROTOCOL_VERSION,
                seq,
            }),
            AppCommand::Render,
        ]
    }

    fn apply_private_sync(&mut self, payload: RemoteSyncPayload) -> Vec<AppCommand> {
        self.match_state.turn_number = self.match_state.turn_number.max(payload.turn_number);
        self.match_state.local_turn = matches!(payload.active_player, RemotePlayer::Remote);
        self.mark_remote_status(ConnectionStatus::Connected);
        if self.local_ships_placed {
            self.state = AppState::Playing;
        } else {
            self.state = AppState::Pairing;
        }
        self.save_request_and_render()
    }

    fn apply_solo_fire(
        &mut self,
        side: PlayerSide,
        coord: (usize, usize),
    ) -> Result<(), BoardError> {
        if (side == PlayerSide::Local) != self.match_state.local_turn {
            return Err(BoardError::UnableToPlaceShip);
        }

        match side {
            PlayerSide::Local => {
                let opponent = self
                    .match_state
                    .opponent_engine
                    .as_mut()
                    .ok_or(BoardError::UnableToPlaceShip)?;
                let result = opponent.opponent_guess(coord.0, coord.1)?;
                self.match_state
                    .local_engine
                    .record_guess(coord.0, coord.1, result)?;
                self.finish_turn(coord, result, true);
            }
            PlayerSide::Opponent => {
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
                self.finish_turn(coord, result, false);
            }
        }
        Ok(())
    }

    fn finish_turn(&mut self, coord: (usize, usize), result: GuessResult, by_local_player: bool) {
        self.match_state.turn_number += 1;
        self.match_state.last_event = Some(GameEventView::Guess {
            coord,
            result,
            by_local_player,
        });
        self.match_state.local_turn = !self.match_state.local_turn;
        self.check_game_over();
    }

    fn check_game_over(&mut self) {
        if matches!(
            self.match_state.local_engine.status(),
            GameStatus::Won | GameStatus::Lost
        ) {
            self.state = AppState::GameOver;
            self.pending_prompt = None;
            self.match_state.last_event = Some(GameEventView::GameOver {
                local_player_won: self.match_state.local_engine.status() == GameStatus::Won,
            });
        }
    }

    fn after_turn_commands(&mut self) -> Vec<AppCommand> {
        let mut commands = if self.state == AppState::GameOver {
            vec![AppCommand::ClearSave]
        } else {
            vec![AppCommand::Save(self.match_state.saved_game())]
        };
        commands.extend(self.observe_commands());
        if self.state == AppState::Playing {
            commands.extend(self.request_current_turn_agent_without_render());
        }
        commands.push(AppCommand::Render);
        commands
    }

    fn save_request_and_render(&mut self) -> Vec<AppCommand> {
        let mut commands = vec![AppCommand::Save(self.match_state.saved_game())];
        if self.state == AppState::Playing {
            commands.extend(self.request_current_turn_agent_without_render());
        }
        commands.push(AppCommand::Render);
        commands
    }

    fn request_current_turn_agent_without_render(&mut self) -> Vec<AppCommand> {
        let side = if self.match_state.local_turn {
            PlayerSide::Local
        } else {
            PlayerSide::Opponent
        };
        if matches!(self.match_mode, MatchMode::Remote(_)) && side == PlayerSide::Opponent {
            self.pending_prompt = None;
            return Vec::new();
        }
        if matches!(
            &self.match_mode,
            MatchMode::Remote(RemoteSession {
                awaiting_status_seq: Some(_),
                ..
            })
        ) {
            self.pending_prompt = None;
            return Vec::new();
        }
        let prompt = AgentPrompt {
            side,
            kind: AgentPromptKind::SelectTarget,
        };
        self.pending_prompt = Some(prompt);
        vec![AppCommand::RequestAgent(prompt)]
    }

    fn observe_commands(&self) -> Vec<AppCommand> {
        let Some(GameEventView::Guess {
            coord,
            result,
            by_local_player,
        }) = self.match_state.last_event
        else {
            return Vec::new();
        };
        let event = GameEvent::GuessResult {
            coord,
            result,
            by_local_player,
        };
        vec![
            AppCommand::RequestAgent(AgentPrompt {
                side: PlayerSide::Local,
                kind: AgentPromptKind::Observe(event),
            }),
            AppCommand::RequestAgent(AgentPrompt {
                side: PlayerSide::Opponent,
                kind: AgentPromptKind::Observe(event),
            }),
        ]
    }

    fn apply_ship_action(
        &mut self,
        side: PlayerSide,
        action: AgentAction,
    ) -> Result<(), BoardError> {
        let placements = match action {
            AgentAction::PlaceShips(placements) => placements,
            _ => return Err(BoardError::UnableToPlaceShip),
        };

        let engine = match side {
            PlayerSide::Local => &mut self.match_state.local_engine,
            PlayerSide::Opponent => self
                .match_state
                .opponent_engine
                .as_mut()
                .ok_or(BoardError::UnableToPlaceShip)?,
        };

        for ShipPlacement {
            ship_index,
            row,
            col,
            orientation,
        } in placements
        {
            engine
                .board_mut()
                .place(ship_index, row, col, orientation)?;
        }
        Ok(())
    }

    fn remote_status(&self) -> Option<ConnectionStatus> {
        match &self.match_mode {
            MatchMode::Remote(session) => Some(session.status),
            MatchMode::Solo => None,
        }
    }

    fn mark_remote_status(&mut self, status: ConnectionStatus) {
        if let MatchMode::Remote(session) = &mut self.match_mode {
            session.status = status;
        } else {
            self.match_mode = MatchMode::Remote(RemoteSession {
                status,
                ..RemoteSession::new(RemoteRole::Host)
            });
        }
    }

    fn remote_next_seq(&mut self) -> u64 {
        match &mut self.match_mode {
            MatchMode::Remote(session) => session.next_seq(),
            MatchMode::Solo => 1,
        }
    }

    fn remote_sync_payload(&self) -> RemoteSyncPayload {
        let state = self.match_state.local_engine.state();
        RemoteSyncPayload {
            turn_number: self.match_state.turn_number,
            active_player: if self.match_state.local_turn {
                RemotePlayer::Local
            } else {
                RemotePlayer::Remote
            },
            next_seq: match &self.match_mode {
                MatchMode::Remote(session) => session.next_seq,
                MatchMode::Solo => 1,
            },
            last_received_seq: match &self.match_mode {
                MatchMode::Remote(session) => session.last_received_seq,
                MatchMode::Solo => None,
            },
            public_shots: GuessBoardState {
                hits: self.match_state.local_engine.guess_hits(),
                misses: self.match_state.local_engine.guess_misses(),
            },
            enemy_ships_remaining: state.enemy_ships_remaining,
            enemy_remaining: state.enemy_remaining,
            status: self.match_state.local_engine.status().into(),
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
    ) -> Result<Vec<AppCommand>, BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        self.match_mode = MatchMode::Solo;
        self.match_state = MatchState::local_ai_game();
        self.local_ships_placed = false;
        self.opponent_ships_placed = false;

        let local_action = self
            .local_agent
            .handle_request(
                AgentRequest::PlaceShips {
                    board: self.match_state.local_engine.board(),
                },
                local_rng,
            )
            .map_err(|_| BoardError::UnableToPlaceShip)?;
        self.apply_ship_action(PlayerSide::Local, local_action)?;

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
        self.apply_ship_action(PlayerSide::Opponent, opponent_action)?;

        self.local_ships_placed = true;
        self.opponent_ships_placed = true;
        self.state = AppState::Playing;
        Ok(self.save_request_and_render())
    }

    pub fn play_next_turn(
        &mut self,
        local_rng: &mut SmallRng,
        opponent_rng: &mut SmallRng,
    ) -> Result<Vec<AppCommand>, BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        if self.state != AppState::Playing {
            return Ok(vec![AppCommand::Render]);
        }

        let side = if self.match_state.local_turn {
            PlayerSide::Local
        } else {
            PlayerSide::Opponent
        };
        let coord = self.agent_target(side, local_rng, opponent_rng)?;
        self.apply_solo_fire(side, coord)?;
        self.notify_agents(local_rng, opponent_rng);
        Ok(self.after_turn_commands())
    }

    fn agent_target(
        &mut self,
        side: PlayerSide,
        local_rng: &mut SmallRng,
        opponent_rng: &mut SmallRng,
    ) -> Result<(usize, usize), BoardError>
    where
        A::Error: core::fmt::Debug,
        O::Error: core::fmt::Debug,
    {
        let (guess_board, remaining) = match side {
            PlayerSide::Local => (
                GuessBoard::from_engine(&self.match_state.local_engine),
                self.match_state.local_engine.enemy_ship_lengths_remaining(),
            ),
            PlayerSide::Opponent => {
                let opponent = self
                    .match_state
                    .opponent_engine
                    .as_ref()
                    .ok_or(BoardError::UnableToPlaceShip)?;
                (
                    GuessBoard::from_engine(opponent),
                    opponent.enemy_ship_lengths_remaining(),
                )
            }
        };

        let action = match side {
            PlayerSide::Local => self
                .local_agent
                .handle_request(
                    AgentRequest::SelectTarget {
                        guess_board: &guess_board,
                        remaining_ships: &remaining,
                    },
                    local_rng,
                )
                .map_err(|_| BoardError::UnableToPlaceShip)?,
            PlayerSide::Opponent => self
                .opponent_agent
                .handle_request(
                    AgentRequest::SelectTarget {
                        guess_board: &guess_board,
                        remaining_ships: &remaining,
                    },
                    opponent_rng,
                )
                .map_err(|_| BoardError::UnableToPlaceShip)?,
        };

        match action {
            AgentAction::Fire(coord) => Ok(coord),
            _ => Err(BoardError::UnableToPlaceShip),
        }
    }

    fn notify_agents(&mut self, local_rng: &mut SmallRng, opponent_rng: &mut SmallRng) {
        let Some(GameEventView::Guess {
            coord,
            result,
            by_local_player,
        }) = self.match_state.last_event
        else {
            return;
        };
        let event = GameEvent::GuessResult {
            coord,
            result,
            by_local_player,
        };
        let _ = self
            .local_agent
            .handle_request(AgentRequest::Observe(event), local_rng);
        let _ = self
            .opponent_agent
            .handle_request(AgentRequest::Observe(event), opponent_rng);
    }
}

fn core_guess_result(res: crate::protocol::domain::GuessResult) -> Result<GuessResult, BoardError> {
    match res {
        crate::protocol::domain::GuessResult::Hit => Ok(GuessResult::Hit),
        crate::protocol::domain::GuessResult::Miss => Ok(GuessResult::Miss),
        crate::protocol::domain::GuessResult::Sink(name) => SHIPS
            .iter()
            .find(|ship| ship.name() == name)
            .map(|ship| GuessResult::Sink(ship.name()))
            .ok_or(BoardError::NameNotFound),
    }
}

fn difficulty_index(difficulty: AiDifficulty) -> usize {
    match difficulty {
        AiDifficulty::Easy => 0,
        AiDifficulty::Medium => 1,
        AiDifficulty::Hard => 2,
        AiDifficulty::Expert => 3,
    }
}

fn difficulty_from_index(index: usize) -> Option<AiDifficulty> {
    match index {
        0 => Some(AiDifficulty::Easy),
        1 => Some(AiDifficulty::Medium),
        2 => Some(AiDifficulty::Hard),
        3 => Some(AiDifficulty::Expert),
        _ => None,
    }
}

fn difficulty_notice(difficulty: AiDifficulty) -> &'static str {
    match difficulty {
        AiDifficulty::Easy => "Current: Easy",
        AiDifficulty::Medium => "Current: Medium",
        AiDifficulty::Hard => "Current: Hard",
        AiDifficulty::Expert => "Current: Expert",
    }
}
