//! Command-line interface utilities and display functions
//!
//! This module provides CLI-related functionality including:
//! - Interface display functions for boards and game state
//! - Experimental CLI runner (incomplete)

#![cfg(feature = "std")]

pub mod interface;

// Re-export interface functions
pub use interface::*;

// Experimental CLI runner
use crate::protocol::GameApi;
use std::io::{self, Write};

pub async fn run_cli(api: Box<dyn GameApi>) -> anyhow::Result<()> {
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        // parse commands like "guess 3 5", "status", etc.
        // This is incomplete and experimental
    }
}
