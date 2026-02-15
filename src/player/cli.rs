#![cfg(feature = "std")]

use std::io::{self, Write};
use std::string::String;

use crate::core::{
    ai,
    bitboard::BitBoard,
    board::Board,
    common::GuessResult,
    config::{BOARD_SIZE, NUM_SHIPS, SHIPS},
    GameEngine,
    BoardError,
};
use rand::rngs::SmallRng;

use super::Player;

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

fn parse_coord(input: &str) -> Result<(usize, usize), String> {
    if input.is_empty() {
        return Err("Empty input".to_string());
    }
    if input.len() < 2 {
        return Err("Too short - need column letter and row number (e.g., A5)".to_string());
    }
    let mut chars = input.chars();
    let col_ch = chars.next().ok_or("No column letter")?.to_ascii_uppercase();
    if !col_ch.is_ascii_alphabetic() {
        return Err(format!("Invalid column '{}' - must be a letter A-J", col_ch));
    }
    let col = (col_ch as u8).wrapping_sub(b'A') as usize;
    if col >= BOARD_SIZE as usize {
        return Err(format!("Column '{}' out of bounds - must be A-J", col_ch));
    }
    let row_str: String = chars.collect();
    let row: usize = row_str.parse()
        .map_err(|_| format!("Invalid row '{}' - must be a number 1-10", row_str))?;
    if row == 0 {
        return Err("Row cannot be 0 - must be 1-10".to_string());
    }
    if row > BOARD_SIZE as usize {
        return Err(format!("Row {} out of bounds - must be 1-10", row));
    }
    Ok((row - 1, col))
}

fn print_board(board: &Board, reveal: bool) {
    std::println!("    ╔═══════════════════════╗");
    std::print!("    ║  ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {}", ch);
    }
    std::println!(" ║");
    std::println!("    ╠═══════════════════════╣");
    for r in 0..BOARD_SIZE as usize {
        std::print!("    ║ {:2}", r + 1);
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
        std::println!(" ║");
    }
    std::println!("    ╚═══════════════════════╝");
    
    // Print legend
    if reveal {
        std::println!("    Legend: S=Ship  X=Hit  o=Miss  .=Water");
    } else {
        std::println!("    Legend: X=Hit  o=Miss  .=Unknown");
    }
    
    // Print ship status
    if reveal {
        std::println!("\n    Ships:");
        let states = board.ship_states();
        for (i, state) in states.iter().enumerate() {
            let def = SHIPS[i];
            let status = if state.sunk { "SUNK" } else { "Active" };
            std::println!("      {} ({}): {}", def.name(), def.length(), status);
        }
    }
}

fn print_guess_board(hits: &BB, misses: &BB) {
    std::println!("    ╔═══════════════════════╗");
    std::print!("    ║  ");
    for c in 0..BOARD_SIZE as usize {
        let ch = (b'A' + c as u8) as char;
        std::print!(" {}", ch);
    }
    std::println!(" ║");
    std::println!("    ╠═══════════════════════╣");
    for r in 0..BOARD_SIZE as usize {
        std::print!("    ║ {:2}", r + 1);
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
        std::println!(" ║");
    }
    std::println!("    ╚═══════════════════════╝");
    std::println!("    Legend: X=Hit  o=Miss  .=Unknown");
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
        std::println!("\n════════════════════════════════════════════════════════════");
        std::println!("                    SHIP PLACEMENT PHASE");
        std::println!("════════════════════════════════════════════════════════════");
        std::println!("\nInstructions:");
        std::println!("  • Enter coordinates and orientation (e.g., A5 H or A5 V)");
        std::println!("  • H = Horizontal, V = Vertical");
        std::println!("  • Valid columns: A-J, Valid rows: 1-10");
        std::println!("  • Press ENTER for random placement");
        std::println!("  • Type 'help' for more information\n");
        
        for i in 0..NUM_SHIPS as usize {
            let def = SHIPS[i];
            loop {
                std::println!("\n═══════════════════════════════════════════════════════════=");
                print_board(board, true);
                std::println!("\nShip {}/{}: {} (length {})", 
                    i + 1, NUM_SHIPS, def.name(), def.length());
                std::print!("Enter placement (or ENTER for random, 'help' for help): ");
                io::stdout().flush().unwrap();
                let mut line = String::new();
                io::stdin().read_line(&mut line).unwrap();
                let line = line.trim();
                
                if line.is_empty() {
                    let (r, c, o) = board.random_placement(rng, i)?;
                    board.place(i, r, c, o)?;
                    std::println!("✓ {} randomly placed at {}", 
                        def.name(), coord_to_string(r, c));
                    break;
                }
                
                if line.eq_ignore_ascii_case("help") {
                    print_placement_help();
                    continue;
                }
                
                let mut parts = line.split_whitespace();
                let coord_str = parts.next();
                let orient_str = parts.next();
                
                if coord_str.is_none() {
                    std::println!("✗ Error: Please enter coordinates (e.g., A5 H)");
                    continue;
                }
                
                let coord_result = parse_coord(coord_str.unwrap());
                let (r, c) = match coord_result {
                    Ok(coord) => coord,
                    Err(e) => {
                        std::println!("✗ Error: {}", e);
                        continue;
                    }
                };
                
                let orient_ch = orient_str
                    .and_then(|s| s.chars().next())
                    .unwrap_or('H');
                    
                let o = if orient_ch == 'v' || orient_ch == 'V' {
                    crate::core::ship::Orientation::Vertical
                } else if orient_ch == 'h' || orient_ch == 'H' {
                    crate::core::ship::Orientation::Horizontal
                } else {
                    std::println!("✗ Error: Invalid orientation '{}' - use H or V", orient_ch);
                    continue;
                };
                
                match board.place(i, r, c, o) {
                    Ok(()) => {
                        std::println!("✓ {} placed successfully at {}", 
                            def.name(), coord_to_string(r, c));
                        break;
                    }
                    Err(e) => {
                        std::println!("✗ Error: {:?}", e);
                        std::println!("   Hint: Make sure the ship fits and doesn't overlap with others.");
                    }
                }
            }
        }
        std::println!("\n✓ All ships placed! Ready to begin battle.\n");
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
            std::print!("\nEnter target coordinates [AI suggests: {}] (or 'help'): ", 
                coord_to_string(sr, sc));
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            let line = line.trim();
            
            if line.is_empty() {
                std::println!("Using AI suggestion: {}", coord_to_string(sr, sc));
                return (sr, sc);
            }
            
            if line.eq_ignore_ascii_case("help") {
                print_targeting_help();
                continue;
            }
            
            match parse_coord(line) {
                Ok((r, c)) => {
                    // Check if already guessed
                    if hits.get(r, c).unwrap_or(false) {
                        std::println!("✗ You already hit that location! Choose another target.");
                        continue;
                    }
                    if misses.get(r, c).unwrap_or(false) {
                        std::println!("✗ You already missed that location! Choose another target.");
                        continue;
                    }
                    return (r, c);
                }
                Err(e) => {
                    std::println!("✗ Invalid coordinate: {}", e);
                    std::println!("   Example: A5, B10, J1");
                }
            }
        }
    }

    fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult) {
        match result {
            GuessResult::Hit => {
                std::println!("\n🎯 HIT! Your shot at {} struck an enemy ship!", 
                    coord_to_string(coord.0, coord.1));
            }
            GuessResult::Miss => {
                std::println!("\n💧 Miss. Your shot at {} hit only water.", 
                    coord_to_string(coord.0, coord.1));
            }
            GuessResult::Sink(name) => {
                std::println!("\n💥 SUNK! You destroyed the enemy's {}!", name);
            }
        }
    }

    fn handle_opponent_guess(&mut self, coord: (usize, usize), result: GuessResult) {
        match result {
            GuessResult::Hit => {
                std::println!("\n⚠️  ENEMY HIT! They struck your ship at {}", 
                    coord_to_string(coord.0, coord.1));
            }
            GuessResult::Miss => {
                std::println!("\n✓ Enemy missed at {}", 
                    coord_to_string(coord.0, coord.1));
            }
            GuessResult::Sink(name) => {
                std::println!("\n💀 SHIP LOST! Enemy destroyed your {} at {}", 
                    name, coord_to_string(coord.0, coord.1));
            }
        }
    }
}

fn print_placement_help() {
    std::println!("\n╔════════════════════════════════════════════════════════╗");
    std::println!("║              SHIP PLACEMENT HELP                       ║");
    std::println!("╠════════════════════════════════════════════════════════╣");
    std::println!("║ Format: <COLUMN><ROW> <ORIENTATION>                    ║");
    std::println!("║                                                        ║");
    std::println!("║ Examples:                                              ║");
    std::println!("║   A5 H    - Place ship at A5, horizontal              ║");
    std::println!("║   B3 V    - Place ship at B3, vertical                ║");
    std::println!("║   J10 H   - Place ship at J10, horizontal             ║");
    std::println!("║                                                        ║");
    std::println!("║ Valid columns: A-J (A is leftmost)                    ║");
    std::println!("║ Valid rows: 1-10 (1 is topmost)                       ║");
    std::println!("║ Orientation: H=Horizontal, V=Vertical                  ║");
    std::println!("║                                                        ║");
    std::println!("║ Ships will extend from the starting position in       ║");
    std::println!("║ the specified direction. Make sure they fit on the    ║");
    std::println!("║ board and don't overlap with other ships.             ║");
    std::println!("║                                                        ║");
    std::println!("║ Press ENTER without input for automatic random        ║");
    std::println!("║ placement of the current ship.                        ║");
    std::println!("╚════════════════════════════════════════════════════════╝\n");
}

fn print_targeting_help() {
    std::println!("\n╔════════════════════════════════════════════════════════╗");
    std::println!("║                  TARGETING HELP                        ║");
    std::println!("╠════════════════════════════════════════════════════════╣");
    std::println!("║ Format: <COLUMN><ROW>                                  ║");
    std::println!("║                                                        ║");
    std::println!("║ Examples:                                              ║");
    std::println!("║   A5     - Target cell A5                             ║");
    std::println!("║   B10    - Target cell B10                            ║");
    std::println!("║   J1     - Target cell J1                             ║");
    std::println!("║                                                        ║");
    std::println!("║ Valid columns: A-J (A is leftmost)                    ║");
    std::println!("║ Valid rows: 1-10 (1 is topmost)                       ║");
    std::println!("║                                                        ║");
    std::println!("║ Board symbols:                                         ║");
    std::println!("║   X = Hit (you struck an enemy ship)                  ║");
    std::println!("║   o = Miss (shot hit water)                           ║");
    std::println!("║   . = Unknown (not yet targeted)                      ║");
    std::println!("║                                                        ║");
    std::println!("║ The AI suggestion is based on probability analysis    ║");
    std::println!("║ of possible ship placements. Press ENTER to use it.   ║");
    std::println!("╚════════════════════════════════════════════════════════╝\n");
}
