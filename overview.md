# Battleship Game Overview

Current snapshot of the Battleship codebase after the adapter refactor.

## Architecture

- `engine`: no_std-compatible game rules, bitboards, boards, ships, game state, and snapshots.
- `agent`: decision makers such as `AiAgent`, `HumanAgent`, `RemoteAgent`, and `ScriptedAgent`.
- `app`: `BattleshipApp`, `MatchState`, app events, and commands for render/save/send actions.
- `input`: platform-neutral `UiEvent` values and input-source traits.
- `render`: passive screen views and renderer trait.
- `render_cli`: std-only CLI input and rendering adapters.
- `protocol`: versioned `WireMessage` values and domain payloads.
- `transport`: app-facing `TransportEndpoint` plus std async TCP, heartbeat, and in-memory adapters.
- `persistence`: adapter-neutral snapshots and `SaveStore` for app runners.
- `data_generation`: feature-gated boundary for AI simulation datasets.

The core game mechanics remain library-first and no_std-compatible. Optional std features provide CLI, TCP, WebSocket, persistence, logging, BLE boundaries, and data-generation support.

## Runtime Flow

`BattleshipApp` owns match orchestration. Agents choose placements or targets when requested; renderers receive passive `ScreenView` values; transports move `WireMessage` payloads; runners decide how to execute emitted `AppCommand` values.

The binary is now a thin CLI runner around a local human-vs-AI `BattleshipApp` game. Network and embedded runners should be built as adapters around the same app-facing traits instead of embedding game logic in transport or UI code.

## Testing

The active test suite covers:

- Engine, board, bitboard, and serialization round trips.
- AI agent target selection and local app game completion.
- App view generation, saved-game restoration, and in-memory save stores.
- In-memory nonblocking transport endpoint behavior.
- TCP framing, malformed frames, cross-version handshakes, fuzz cases, and transport resilience.

Legacy mixed-player and RPC tests were removed with the old architecture.

## Current Feature Gates

- `default = ["std", "cli", "ai", "tcp", "logging"]`
- `std`: standard-library build support for current async/protocol code.
- `cli`: CLI runner and renderer.
- `tcp`: TCP transport support.
- `in-memory`: in-memory transport support for tests/runners.
- `persistence`: app-facing save snapshots and stores.
- `websocket`, `web`, `ble`, `esp-idf`, `data-generation`, `logging`: optional adapter boundaries.

## Next Work

- Split `std` dependencies more strictly so `std` means only standard-library support.
- Expand WebSocket, BLE, and persistence backends beyond their adapter boundaries.
- Add a remote-game runner that bridges async transports into `TransportEndpoint` polling.
- Add renderer snapshots for CLI and future LCD/web renderers.
