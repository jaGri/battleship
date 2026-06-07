# Test Guidelines For Agents

These guidelines apply to tracked integration tests in this directory.

## Test Map

- `board_tests.rs`, `bitboard_tests.rs`, `board_props.rs`, and `game_engine_props.rs`: core board, bitboard, ship, and game-rule behavior.
- `ai_difficulty_tests.rs` and `ai_game_tests.rs`: `AiAgent` difficulty behavior and local AI app completion.
- `app_state_machine_tests.rs`, `adapter_architecture_tests.rs`, and `remote_play_tests.rs`: `BattleshipApp`, passive views, save-store behavior, remote-play runner behavior, and adapter boundaries.
- `game_state_serialization.rs`, `cross_version_tests.rs`, `fuzz_bincode_tests.rs`, and `malformed_frame_tests.rs`: protocol serialization, versioning, and malformed-frame resilience.
- `transport_resilience_tests.rs`: transport endpoint resilience, TCP behavior, heartbeat behavior, and in-memory transport behavior.

## Naming

- Prefer agent terminology over old interface terminology.
- Name new files by subsystem and behavior, for example `remote_play_tests.rs` or `cli_renderer_tests.rs`.
- Keep test names focused on observable behavior instead of implementation history.

## Feature Expectations

- Use `#![cfg(feature = "std")]` for tests that need std-only adapters, async runtimes, serialization, or persistence stores.
- Keep core rule tests compatible with no-default-features whenever possible.
- When adding property tests, cap case counts so the primary suite remains fast.
- Mark slow stress tests with `#[ignore]`.

## Verification

- Run focused tests for code changes in the subsystem you touched.
- Run `cargo check --no-default-features` after touching core, app, protocol, render, or input code.
- Run `cargo test --all-features` when changing adapter behavior, public API shape, or feature gates.
- Skip test runs for documentation-only changes.
