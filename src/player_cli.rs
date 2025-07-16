#![cfg(feature = "std")]

use std::io::{self, Write};
use std::string::String;

use crate::{
    ai,
    bitboard::BitBoard,
    board::Board,
    common::GuessResult,
    config::{BOARD_SIZE, NUM_SHIPS, SHIPS},
    GameEngine,
    BoardError,
};
use rand::rngs::SmallRng;

use crate::player::Player;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

pub struct CliPlayer;

impl CliPlayer {
    pub fn new() -> Self {
        Self
    }
}

fn coord_to_string(r: usize, c: usize) -> String {
    let col = (b'A' + c as u8) as char;
    std::format!("{}{}", col, r + 1)
}

fn parse_coord(input: &str) -> Option<(usize, usize)> {
    if input.len() < 2 {
        return None;
    }
    let mut chars = input.chars();
    let col_ch = chars.next()?.to_ascii_uppercase();
    let col = (col_ch as u8).wrapping_sub(b'A') as usize;
    let row_str: String = chars.collect();
    let row: usize = row_str.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((row - 1, col))
}

fn print_board(board: &Board, reveal: bool) {
    std::print!("   ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {}", ch);
    }
    std::println!();
    for r in 0..BOARD_SIZE as usize {
        std::print!("{:2} ", r + 1);
        for c in 0..BOARD_SIZE as usize {
            let ch = if board.hits().get(r, c).unwrap_or(false) {
                'X'
            } else if board.misses().get(r, c).unwrap_or(false) {
                'o'
            } else if reveal && board.ship_map().get(r, c).unwrap_or(false) {
                'S'
            } else {
                '.'
            };
            std::print!(" {}", ch);
        }
        std::println!();
    }
}

fn print_guess_board(hits: &BB, misses: &BB) {
    std::print!("   ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {}", ch);
    }
    std::println!();
    for r in 0..BOARD_SIZE as usize {
        std::print!("{:2} ", r + 1);
        for c in 0..BOARD_SIZE as usize {
            let ch = if hits.get(r, c).unwrap_or(false) {
                'X'
            } else if misses.get(r, c).unwrap_or(false) {
                'o'
            } else {
                '.'
            };
            std::print!(" {}", ch);
        }
        std::println!();
    }
}

/// Print a normalized probability distribution matrix.
pub fn print_probability_board(pdf: &[[f64; BOARD_SIZE as usize]; BOARD_SIZE as usize]) {
    std::println!("\nProbability distribution:");
    std::print!("   ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {:>4}", ch);
    }
    std::println!();
    for r in 0..BOARD_SIZE as usize {
        std::print!("{:2} ", r + 1);
        for c in 0..BOARD_SIZE as usize {
            std::print!(" {:4.2}", pdf[r][c]);
        }
        std::println!();
    }
}

/// Display the opponent board (top) and the player's board (bottom).
pub fn print_player_view(engine: &GameEngine) {
    std::println!("Opponent board:");
    print_guess_board(&engine.guess_hits(), &engine.guess_misses());
    std::println!("\nYour board:");
    print_board(engine.board(), true);
}

impl Player for CliPlayer {
    fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError> {
        std::println!("Place your ships (e.g. A5 H). Press enter for random placement.");
        for i in 0..NUM_SHIPS as usize {
            let def = SHIPS[i];
            loop {
                print_board(board, true);
                std::print!("Place {} (length {}): ", def.name(), def.length());
                io::stdout().flush().unwrap();
                let mut line = String::new();
                io::stdin().read_line(&mut line).unwrap();
                let line = line.trim();
                if line.is_empty() {
                    let (r, c, o) = board.random_placement(rng, i)?;
                    board.place(i, r, c, o)?;
                    break;
                }
                let mut parts = line.split_whitespace();
                let coord = parts.next().and_then(parse_coord);
                let orient = parts.next().map(|p| p.chars().next().unwrap_or('H'));
                if let (Some((r, c)), Some(o)) = (coord, orient) {
                    let o = if o == 'v' || o == 'V' {
                        crate::ship::Orientation::Vertical
                    } else {
                        crate::ship::Orientation::Horizontal
                    };
                    match board.place(i, r, c, o) {
                        Ok(()) => break,
                        Err(e) => std::println!("Error: {:?}", e),
                    }
                } else {
                    std::println!("Invalid input");
                }
            }
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
        let (sr, sc) = ai::calc_pdf_and_guess(hits, misses, remaining, rng);
        loop {
            // Show probability-based suggestion in brackets
            std::print!("Enter guess [{}]: ", coord_to_string(sr, sc));
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            let line = line.trim();
            if line.is_empty() {
                return (sr, sc);
            }
            if let Some((r, c)) = parse_coord(line) {
                return (r, c);
            } else {
                std::println!("Invalid coordinate");
            }
        }
    }

    fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult) {
        std::println!(
            "You guessed {} -> {:?}",
            coord_to_string(coord.0, coord.1),
            result
        );
    }

    fn handle_opponent_guess(&mut self, coord: (usize, usize), result: GuessResult) {
        std::println!(
            "Opponent guessed {} -> {:?}",
            coord_to_string(coord.0, coord.1),
            result
        );
    }
}
