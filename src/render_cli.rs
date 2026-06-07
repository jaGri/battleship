//! CLI input and renderer adapters for the app architecture.

use std::io::{self, Write};

use crate::agent::Coordinate;
use crate::engine::{Board, GuessBoard, GuessResult, BOARD_SIZE};
use crate::input::{InputSource, UiEvent};
use crate::render::{GameEventView, Renderer, ScreenView};

/// Blocking command-line input adapter.
pub struct CliInput;

impl CliInput {
    pub fn new() -> Self {
        Self
    }

    pub fn coord_to_string(row: usize, col: usize) -> String {
        let col_char = (b'A' + col as u8) as char;
        format!("{}{}", col_char, row + 1)
    }

    fn parse_coord(input: &str) -> Option<Coordinate> {
        let trimmed = input.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() < 2 {
            return None;
        }

        let col = match bytes[0].to_ascii_uppercase() {
            b'A'..=b'J' => (bytes[0].to_ascii_uppercase() - b'A') as usize,
            _ => return None,
        };
        let row: usize = trimmed[1..].parse().ok()?;
        if row == 0 || row > BOARD_SIZE as usize || col >= BOARD_SIZE as usize {
            return None;
        }
        Some((row - 1, col))
    }
}

impl Default for CliInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSource for CliInput {
    type Error = io::Error;

    fn poll_input(&mut self) -> Result<Option<UiEvent>, Self::Error> {
        print!("Enter target coordinate (A1-J10): ");
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        Self::parse_coord(trimmed).map(|coord| Some(UiEvent::Target(coord))).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid coordinate: {}", trimmed),
            )
        })
    }
}

/// Command-line renderer for passive screen views.
pub struct CliRenderer;

impl CliRenderer {
    pub fn new() -> Self {
        Self
    }

    fn print_board(board: &Board, reveal_ships: bool) {
        print!("   ");
        for c in 0..BOARD_SIZE as usize {
            let ch = (b'A' + c as u8) as char;
            print!(" {}", ch);
        }
        println!();

        for r in 0..BOARD_SIZE as usize {
            print!("{:2} ", r + 1);
            for c in 0..BOARD_SIZE as usize {
                let ch = if board.hits().get(r, c).unwrap_or(false) {
                    'X'
                } else if board.misses().get(r, c).unwrap_or(false) {
                    'o'
                } else if reveal_ships && board.ship_map().get(r, c).unwrap_or(false) {
                    'S'
                } else {
                    '.'
                };
                print!(" {}", ch);
            }
            println!();
        }
    }

    fn print_guess_board(guess_board: &GuessBoard) {
        print!("   ");
        for c in 0..BOARD_SIZE as usize {
            let ch = (b'A' + c as u8) as char;
            print!(" {}", ch);
        }
        println!();

        for r in 0..BOARD_SIZE as usize {
            print!("{:2} ", r + 1);
            for c in 0..BOARD_SIZE as usize {
                let ch = if guess_board.hits.get(r, c).unwrap_or(false) {
                    'X'
                } else if guess_board.misses.get(r, c).unwrap_or(false) {
                    'o'
                } else {
                    '.'
                };
                print!(" {}", ch);
            }
            println!();
        }
    }

    fn print_event(event: GameEventView) {
        match event {
            GameEventView::Guess {
                coord,
                result,
                by_local_player,
            } => {
                let who = if by_local_player { "You" } else { "Opponent" };
                let coord = CliInput::coord_to_string(coord.0, coord.1);
                match result {
                    GuessResult::Hit => println!("{} hit at {}.", who, coord),
                    GuessResult::Miss => println!("{} missed at {}.", who, coord),
                    GuessResult::Sink(ship) => println!("{} sank {} at {}.", who, ship, coord),
                }
            }
            GameEventView::GameOver { local_player_won } => {
                if local_player_won {
                    println!("Game over: you won.");
                } else {
                    println!("Game over: you lost.");
                }
            }
        }
    }
}

impl Default for CliRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CliRenderer {
    type Error = io::Error;

    fn render(&mut self, view: &ScreenView<'_>) -> Result<(), Self::Error> {
        match view {
            ScreenView::Game(game) => {
                println!("\n=== Turn {} ===", game.turn_number);
                if let Some(event) = game.last_event {
                    Self::print_event(event);
                }
                println!("\nOpponent board:");
                Self::print_guess_board(&game.guess_board);
                println!("\nYour board:");
                Self::print_board(game.my_board, true);
                println!(
                    "\n{}",
                    if game.my_turn {
                        "Your turn."
                    } else {
                        "Waiting for opponent."
                    }
                );
            }
            ScreenView::Message(message) => {
                println!("{}\n{}", message.title, message.body);
            }
            ScreenView::Title => println!("Battleship"),
            ScreenView::Menu(menu) => {
                println!("{}", menu.title);
                for (idx, item) in menu.items.iter().enumerate() {
                    println!("{} {}", if idx == menu.selected { ">" } else { " " }, item);
                }
            }
            ScreenView::Pairing(connection) | ScreenView::ConnectionOverlay(connection) => {
                println!(
                    "Connection: {} ({})",
                    if connection.connected {
                        "connected"
                    } else {
                        "offline"
                    },
                    connection.status
                );
                if let Some(code) = connection.code {
                    println!("Code: {}", code);
                }
            }
        }
        Ok(())
    }
}
