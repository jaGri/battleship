//! Passive view models and renderer trait.

use crate::agent::Coordinate;
use crate::engine::{Board, GameStatus, GuessBoard, GuessResult};

/// Top-level screen the app wants displayed.
pub enum ScreenView<'a> {
    Title,
    Menu(MenuView<'a>),
    Game(GameView<'a>),
    Pairing(ConnectionView<'a>),
    ConnectionOverlay(ConnectionView<'a>),
    Message(MessageView<'a>),
}

/// Menu view model.
pub struct MenuView<'a> {
    pub title: &'a str,
    pub items: &'a [&'a str],
    pub selected: usize,
}

/// In-game view model.
pub struct GameView<'a> {
    pub my_board: &'a Board,
    pub guess_board: GuessBoard,
    pub my_turn: bool,
    pub turn_number: u32,
    pub status: GameStatus,
    pub last_event: Option<GameEventView>,
}

/// Connection-related view model.
pub struct ConnectionView<'a> {
    pub code: Option<&'a str>,
    pub connected: bool,
    pub status: &'a str,
}

/// Simple message screen view model.
pub struct MessageView<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

/// Last noteworthy game event for renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventView {
    Guess {
        coord: Coordinate,
        result: GuessResult,
        by_local_player: bool,
    },
    GameOver {
        local_player_won: bool,
    },
}

/// Renderer for passive views.
pub trait Renderer {
    type Error;

    fn render(&mut self, view: &ScreenView<'_>) -> Result<(), Self::Error>;
}
