//! Platform-neutral input events and sources.

use crate::agent::Coordinate;

/// UI events emitted by keyboards, buttons, web clients, or tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Start,
    ConnectionMenu,
    Tick,
    Target(Coordinate),
}

/// Source of normalized UI events.
pub trait InputSource {
    type Error;

    fn poll_input(&mut self) -> Result<Option<UiEvent>, Self::Error>;
}
