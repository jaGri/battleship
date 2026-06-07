# Battleship

Rust Battleship implementation with a no_std-compatible rules engine and adapter-based app architecture for AI, CLI, transport, rendering, and persistence experiments.

## Module Layout

- `engine`: board, bitboard, ship, and game rules.
- `agent`: AI, human, remote, and scripted decision makers.
- `app`: `BattleshipApp`, match state, app events, and runner commands.
- `input`: platform-neutral UI events.
- `render`: passive screen views and renderer trait.
- `render_cli`: CLI input and rendering adapters.
- `web`: web input normalization and owned web view models.
- `protocol`: versioned `WireMessage` payloads.
- `transport`: nonblocking app endpoint plus TCP, heartbeat, in-memory, WebSocket, and BLE boundaries.
- `persistence`: saved-game snapshots, `SaveStore`, and file-backed active saves.
- `data_generation`: optional AI simulation data boundary.

## Commands

Build:

```bash
cargo build
```

Run a local human-vs-AI game:

```bash
cargo run -- --seed 123 --difficulty hard
```

The CLI runner saves active games to `battleship.sav` in the current working
directory. Use "Resume Game" from the main menu to load that active save; the
file is cleared when a game ends.

Run the default test suite:

```bash
cargo test
```

Run the full feature test suite:

```bash
cargo test --all-features
```

Check no_std compatibility:

```bash
cargo check --no-default-features
```

Check all feature combinations used by the current adapter boundaries:

```bash
cargo check --all-features
```

Run clippy when changing Rust code:

```bash
cargo clippy --all-features -- -D warnings
```
