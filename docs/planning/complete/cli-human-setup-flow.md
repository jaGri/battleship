# CLI Human Setup Flow

Make the CLI app feel like a complete game rather than a direct battle loop.

## Completion Summary

- The CLI now starts through app-owned title and menu states instead of jumping directly into the battle loop.
- Solo setup supports random placement with accept, reroll, and manual fallback.
- Manual placement accepts a coordinate plus orientation for each ship and previews the board after each valid placement.
- The main menu includes new solo game, resume game, remote host, remote join, difficulty, and quit.
- Difficulty selection emits `AppCommand::ConfigureDifficulty`, keeping AI configuration runner-owned.
- CLI input loops recover from invalid setup and target input without panicking or advancing game state.

## Verification

- `cargo fmt --check`
- `cargo test --all-features`
- `cargo check --no-default-features`
- `cargo clippy --all-features -- -D warnings`

## Resolution

- Implementation is already present in the tracked app, CLI runner, renderer/input adapter, human agent, and app state-machine tests.
- No public API, CLI flag, protocol, persistence, README, or overview changes were needed to complete this plan.
