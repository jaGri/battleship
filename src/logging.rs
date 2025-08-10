#![cfg(feature = "std")]

use std::env;
use log::{self, LevelFilter, Metadata, Record};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

/// Initialize logging with a level taken from the `BATTLESHIP_LOG` environment variable.
/// Defaults to `info` if the variable is not set or invalid.
pub fn init_logging() {
    let level = env::var("BATTLESHIP_LOG")
        .ok()
        .and_then(|lvl| lvl.parse().ok())
        .unwrap_or(LevelFilter::Info);
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(level));
}
