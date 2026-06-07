//! Platform-neutral input events and sources.

use crate::agent::Coordinate;
use crate::engine::Orientation;

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
    RandomPlacement,
    ClearPlacements,
    PlaceShip {
        ship_index: usize,
        row: usize,
        col: usize,
        orientation: Orientation,
    },
}

/// Source of normalized UI events.
pub trait InputSource {
    type Error;

    fn poll_input(&mut self) -> Result<Option<UiEvent>, Self::Error>;
}
