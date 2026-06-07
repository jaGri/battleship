//! Web-facing input and render adapters.
//!
//! Browser and server runners should normalize browser actions into
//! [`WebInputEvent`] values, feed emitted [`UiEvent`](crate::input::UiEvent)
//! values into [`BattleshipApp`](crate::app::BattleshipApp), and bridge
//! websocket/network frames separately as protocol
//! [`WireMessage`](crate::protocol::WireMessage) values. Outbound
//! [`AppCommand::Send`](crate::app::AppCommand::Send) commands should be
//! executed by the runner's transport layer, not this module.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::string::{String, ToString};
use std::vec::Vec;

use crate::engine::{
    Board, GameStatus, GuessBoard, GuessResult, Orientation, BOARD_SIZE, NUM_SHIPS, SHIPS,
};
use crate::input::{InputSource, UiEvent};
use crate::render::{
    ConnectionView, GameEventView, GameView, MenuView, MessageView, Renderer, ScreenView,
};

/// Browser-originated UI event before normalization into an app event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebInputEvent {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Start,
    ConnectionMenu,
    Tick,
    Target {
        row: usize,
        col: usize,
    },
    RandomPlacement,
    ClearPlacements,
    PlaceShip {
        ship_index: usize,
        row: usize,
        col: usize,
        orientation: WebOrientation,
    },
}

/// Error returned when a browser event cannot be normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebInputError {
    TargetOutOfBounds { row: usize, col: usize },
    PlacementOutOfBounds { row: usize, col: usize },
    ShipIndexOutOfBounds { ship_index: usize },
}

impl fmt::Display for WebInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetOutOfBounds { row, col } => {
                write!(f, "target out of bounds: row={}, col={}", row, col)
            }
            Self::PlacementOutOfBounds { row, col } => {
                write!(f, "placement out of bounds: row={}, col={}", row, col)
            }
            Self::ShipIndexOutOfBounds { ship_index } => {
                write!(f, "ship index out of bounds: ship_index={}", ship_index)
            }
        }
    }
}

impl Error for WebInputError {}

/// Queue-backed web input adapter.
#[derive(Debug, Default)]
pub struct WebInput {
    events: VecDeque<UiEvent>,
}

impl WebInput {
    /// Create an empty web input queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalize and enqueue a browser-originated event.
    pub fn push_event(&mut self, event: WebInputEvent) -> Result<(), WebInputError> {
        let event = normalize_input_event(event)?;
        self.events.push_back(event);
        Ok(())
    }

    /// Return the number of queued normalized events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true when there are no queued events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl InputSource for WebInput {
    type Error = WebInputError;

    fn poll_input(&mut self) -> Result<Option<UiEvent>, Self::Error> {
        Ok(self.events.pop_front())
    }
}

fn normalize_input_event(event: WebInputEvent) -> Result<UiEvent, WebInputError> {
    match event {
        WebInputEvent::Up => Ok(UiEvent::Up),
        WebInputEvent::Down => Ok(UiEvent::Down),
        WebInputEvent::Left => Ok(UiEvent::Left),
        WebInputEvent::Right => Ok(UiEvent::Right),
        WebInputEvent::Confirm => Ok(UiEvent::Confirm),
        WebInputEvent::Back => Ok(UiEvent::Back),
        WebInputEvent::Start => Ok(UiEvent::Start),
        WebInputEvent::ConnectionMenu => Ok(UiEvent::ConnectionMenu),
        WebInputEvent::Tick => Ok(UiEvent::Tick),
        WebInputEvent::Target { row, col } => {
            if row < BOARD_SIZE as usize && col < BOARD_SIZE as usize {
                Ok(UiEvent::Target((row, col)))
            } else {
                Err(WebInputError::TargetOutOfBounds { row, col })
            }
        }
        WebInputEvent::RandomPlacement => Ok(UiEvent::RandomPlacement),
        WebInputEvent::ClearPlacements => Ok(UiEvent::ClearPlacements),
        WebInputEvent::PlaceShip {
            ship_index,
            row,
            col,
            orientation,
        } => {
            if ship_index >= NUM_SHIPS {
                return Err(WebInputError::ShipIndexOutOfBounds { ship_index });
            }
            if row >= BOARD_SIZE as usize || col >= BOARD_SIZE as usize {
                return Err(WebInputError::PlacementOutOfBounds { row, col });
            }
            Ok(UiEvent::PlaceShip {
                ship_index,
                row,
                col,
                orientation: Orientation::from(orientation),
            })
        }
    }
}

/// Renderer that stores the latest owned web view model.
#[derive(Debug, Default)]
pub struct WebRenderer {
    latest: Option<WebScreenView>,
}

impl WebRenderer {
    /// Create an empty web renderer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Latest rendered web view, if any.
    pub fn latest(&self) -> Option<&WebScreenView> {
        self.latest.as_ref()
    }

    /// Consume and return the latest rendered web view, if any.
    pub fn take_latest(&mut self) -> Option<WebScreenView> {
        self.latest.take()
    }
}

impl Renderer for WebRenderer {
    type Error = Infallible;

    fn render(&mut self, view: &ScreenView<'_>) -> Result<(), Self::Error> {
        self.latest = Some(WebScreenView::from(view));
        Ok(())
    }
}

/// Owned top-level web view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebScreenView {
    Title,
    Menu(WebMenuView),
    Game(WebGameView),
    Pairing(WebConnectionView),
    ConnectionOverlay(WebConnectionView),
    Message(WebMessageView),
}

impl From<&ScreenView<'_>> for WebScreenView {
    fn from(view: &ScreenView<'_>) -> Self {
        match view {
            ScreenView::Title => Self::Title,
            ScreenView::Menu(view) => Self::Menu(WebMenuView::from(view)),
            ScreenView::Game(view) => Self::Game(WebGameView::from(view)),
            ScreenView::Pairing(view) => Self::Pairing(WebConnectionView::from(view)),
            ScreenView::ConnectionOverlay(view) => {
                Self::ConnectionOverlay(WebConnectionView::from(view))
            }
            ScreenView::Message(view) => Self::Message(WebMessageView::from(view)),
        }
    }
}

/// Owned web menu view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebMenuView {
    pub title: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub notice: Option<String>,
}

impl From<&MenuView<'_>> for WebMenuView {
    fn from(view: &MenuView<'_>) -> Self {
        Self {
            title: view.title.to_string(),
            items: view.items.iter().map(|item| item.to_string()).collect(),
            selected: view.selected,
            notice: view.notice.map(ToString::to_string),
        }
    }
}

/// Owned web game view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebGameView {
    pub my_board: WebBoardView,
    pub guess_board: WebGuessBoardView,
    pub my_turn: bool,
    pub turn_number: u32,
    pub status: WebGameStatus,
    pub last_event: Option<WebGameEvent>,
}

impl From<&GameView<'_>> for WebGameView {
    fn from(view: &GameView<'_>) -> Self {
        Self {
            my_board: WebBoardView::from_board(view.my_board),
            guess_board: WebGuessBoardView::from_guess_board(&view.guess_board),
            my_turn: view.my_turn,
            turn_number: view.turn_number,
            status: WebGameStatus::from(view.status),
            last_event: view.last_event.map(WebGameEvent::from),
        }
    }
}

/// Owned web connection view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebConnectionView {
    pub code: Option<String>,
    pub connected: bool,
    pub status: String,
}

impl From<&ConnectionView<'_>> for WebConnectionView {
    fn from(view: &ConnectionView<'_>) -> Self {
        Self {
            code: view.code.map(ToString::to_string),
            connected: view.connected,
            status: view.status.to_string(),
        }
    }
}

/// Owned web message view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebMessageView {
    pub title: String,
    pub body: String,
}

impl From<&MessageView<'_>> for WebMessageView {
    fn from(view: &MessageView<'_>) -> Self {
        Self {
            title: view.title.to_string(),
            body: view.body.to_string(),
        }
    }
}

/// Owned local-board view for web clients.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebBoardView {
    pub size: usize,
    pub cells: Vec<Vec<WebBoardCell>>,
    pub ships: Vec<WebShipView>,
}

impl WebBoardView {
    /// Build a web board view from the local board.
    pub fn from_board(board: &Board) -> Self {
        let mut cells = Vec::with_capacity(BOARD_SIZE as usize);
        for row in 0..BOARD_SIZE as usize {
            let mut row_cells = Vec::with_capacity(BOARD_SIZE as usize);
            for col in 0..BOARD_SIZE as usize {
                row_cells.push(board_cell(board, row, col));
            }
            cells.push(row_cells);
        }

        let ships = board
            .ship_states()
            .into_iter()
            .enumerate()
            .map(|(ship_index, state)| WebShipView {
                name: state.name.to_string(),
                length: SHIPS[ship_index].length(),
                sunk: state.sunk,
                placement: state
                    .position
                    .map(|(row, col, orientation)| WebShipPlacement {
                        row,
                        col,
                        orientation: WebOrientation::from(orientation),
                    }),
            })
            .collect();

        Self {
            size: BOARD_SIZE as usize,
            cells,
            ships,
        }
    }
}

fn board_cell(board: &Board, row: usize, col: usize) -> WebBoardCell {
    if board.misses().get(row, col).unwrap_or(false) {
        WebBoardCell::Miss
    } else if is_sunk_ship_cell(board, row, col) {
        WebBoardCell::Sunk
    } else if board.hits().get(row, col).unwrap_or(false) {
        WebBoardCell::Hit
    } else if board.ship_map().get(row, col).unwrap_or(false) {
        WebBoardCell::Ship
    } else {
        WebBoardCell::Water
    }
}

fn is_sunk_ship_cell(board: &Board, row: usize, col: usize) -> bool {
    board
        .ship_states()
        .into_iter()
        .enumerate()
        .any(|(ship_index, state)| {
            state.sunk
                && state
                    .position
                    .map(|(ship_row, ship_col, orientation)| {
                        ship_covers(
                            ship_row,
                            ship_col,
                            orientation,
                            SHIPS[ship_index].length(),
                            row,
                            col,
                        )
                    })
                    .unwrap_or(false)
        })
}

fn ship_covers(
    ship_row: usize,
    ship_col: usize,
    orientation: Orientation,
    length: usize,
    row: usize,
    col: usize,
) -> bool {
    match orientation {
        Orientation::Horizontal => row == ship_row && col >= ship_col && col < ship_col + length,
        Orientation::Vertical => col == ship_col && row >= ship_row && row < ship_row + length,
    }
}

/// Local board cell state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebBoardCell {
    Water,
    Ship,
    Hit,
    Miss,
    Sunk,
}

/// Owned guess-board view for web clients.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebGuessBoardView {
    pub size: usize,
    pub cells: Vec<Vec<WebGuessCell>>,
}

impl WebGuessBoardView {
    /// Build a web guess-board view from public guess history.
    pub fn from_guess_board(board: &GuessBoard) -> Self {
        let mut cells = Vec::with_capacity(BOARD_SIZE as usize);
        for row in 0..BOARD_SIZE as usize {
            let mut row_cells = Vec::with_capacity(BOARD_SIZE as usize);
            for col in 0..BOARD_SIZE as usize {
                row_cells.push(guess_cell(board, row, col));
            }
            cells.push(row_cells);
        }

        Self {
            size: BOARD_SIZE as usize,
            cells,
        }
    }
}

fn guess_cell(board: &GuessBoard, row: usize, col: usize) -> WebGuessCell {
    if board.misses.get(row, col).unwrap_or(false) {
        WebGuessCell::Miss
    } else if board.hits.get(row, col).unwrap_or(false) {
        if board.active_hits.get(row, col).unwrap_or(false) {
            WebGuessCell::ActiveHit
        } else {
            WebGuessCell::Hit
        }
    } else {
        WebGuessCell::Unknown
    }
}

/// Guess-board cell state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebGuessCell {
    Unknown,
    Hit,
    ActiveHit,
    Miss,
}

/// Owned ship view for web clients.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebShipView {
    pub name: String,
    pub length: usize,
    pub sunk: bool,
    pub placement: Option<WebShipPlacement>,
}

/// Owned ship placement for web clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WebShipPlacement {
    pub row: usize,
    pub col: usize,
    pub orientation: WebOrientation,
}

/// Owned orientation for web clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebOrientation {
    Horizontal,
    Vertical,
}

impl From<Orientation> for WebOrientation {
    fn from(orientation: Orientation) -> Self {
        match orientation {
            Orientation::Horizontal => Self::Horizontal,
            Orientation::Vertical => Self::Vertical,
        }
    }
}

impl From<WebOrientation> for Orientation {
    fn from(orientation: WebOrientation) -> Self {
        match orientation {
            WebOrientation::Horizontal => Self::Horizontal,
            WebOrientation::Vertical => Self::Vertical,
        }
    }
}

/// Owned game status for web clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebGameStatus {
    InProgress,
    Won,
    Lost,
}

impl From<GameStatus> for WebGameStatus {
    fn from(status: GameStatus) -> Self {
        match status {
            GameStatus::InProgress => Self::InProgress,
            GameStatus::Won => Self::Won,
            GameStatus::Lost => Self::Lost,
        }
    }
}

/// Owned guess result for web clients.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebGuessResult {
    Hit,
    Miss,
    Sink { ship: String },
}

impl From<GuessResult> for WebGuessResult {
    fn from(result: GuessResult) -> Self {
        match result {
            GuessResult::Hit => Self::Hit,
            GuessResult::Miss => Self::Miss,
            GuessResult::Sink(ship) => Self::Sink {
                ship: ship.to_string(),
            },
        }
    }
}

/// Owned last-event view for web clients.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum WebGameEvent {
    Guess {
        row: usize,
        col: usize,
        result: WebGuessResult,
        by_local_player: bool,
    },
    GameOver {
        local_player_won: bool,
    },
}

impl From<GameEventView> for WebGameEvent {
    fn from(event: GameEventView) -> Self {
        match event {
            GameEventView::Guess {
                coord,
                result,
                by_local_player,
            } => Self::Guess {
                row: coord.0,
                col: coord.1,
                result: WebGuessResult::from(result),
                by_local_player,
            },
            GameEventView::GameOver { local_player_won } => Self::GameOver { local_player_won },
        }
    }
}
