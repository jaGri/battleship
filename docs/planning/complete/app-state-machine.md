# App State Machine

Turn `BattleshipApp::update()` into the central reducer for product behavior.

## Current State

- `AppState`, `AppEvent`, and `AppCommand` exist.
- `BattleshipApp::place_ships()` and `play_next_turn()` can run a local game.
- `update()` currently emits only `AppCommand::Render`.

## Next Questions

- What events should drive title, menu, setup, pairing, playing, and game-over screens?
- Should `play_next_turn()` be folded into `update()` or remain a helper?
- What should command batching look like for render, send, save, clear-save, and exit?

## Candidate Work

- Define full `AppEvent` handling for UI, transport, tick, and persistence events.
- Add state transitions for title, main menu, solo setup, pairing, playing, connection overlay, and game over.
- Replace bare `AppCommand::Save` with a command that carries save data.
- Add tests for every state transition.
