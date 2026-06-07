# Battleship

Rust Battleship implementation with a no_std-compatible rules engine and adapter-based app architecture for AI, CLI, transport, rendering, and persistence experiments.

## Module Layout

- `engine`: board, bitboard, ship, and game rules.
- `agent`: AI, human, remote, and scripted decision makers.
- `app`: `BattleshipApp`, match state, app events, and runner commands.
- `input`: platform-neutral UI events.
- `render`: passive screen views and renderer trait.
- `render_cli`: CLI input and rendering adapters.
- `protocol`: versioned `WireMessage` payloads.
- `transport`: nonblocking app endpoint plus TCP, heartbeat, in-memory, WebSocket, and BLE boundaries.
- `persistence`: saved-game snapshots and `SaveStore`.
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

Run tests:

```bash
cargo test --all-features
```

Check no_std compatibility:

```bash
cargo check --no-default-features
```
