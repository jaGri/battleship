# CLI Human Setup Flow

Make the CLI app feel like a complete game rather than a direct battle loop.

## Current State

- `CliInput` can parse target coordinates.
- `CliRenderer` can render game views.
- `HumanAgent` currently places ships randomly.
- `main.rs` starts directly in human-vs-AI mode.

## Next Questions

- Should ship placement be manual, random with confirmation, or both?
- What menu options should exist for solo, resume, difficulty, and quit?
- Should CLI commands be separate `UiEvent`s or parsed in app state?

## Candidate Work

- Add CLI menu rendering and selection input.
- Add manual or confirmable ship placement.
- Add difficulty selection.
- Add resume/new-game prompt once persistence is wired.
- Improve invalid-input recovery so the app loop owns retry behavior.
