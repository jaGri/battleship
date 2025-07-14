#![cfg(feature = "std")]

//! Experimental text-based client interface.
//! This module is incomplete and may change without notice.
//! It is only compiled when the `std` feature is enabled.

use crate::protocol::GameApi;
use std::io::{self, Write};

pub async fn run_cli(api: Box<dyn GameApi>) -> anyhow::Result<()> {
    loop {
        print!("> "); io::stdout().flush()?;
        let mut buf = String::new(); io::stdin().read_line(&mut buf)?;
        // parse commands like "guess 3 5", "status", etc.
    }
}
