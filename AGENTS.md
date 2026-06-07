# Agent Guidelines for Battleship

Guidelines for AI agents working on this Rust Battleship implementation.

## Project Context

This is a Rust Battleship implementation designed for adapter experiments across CLI,
network, web, and embedded targets. The codebase is library-first: core game rules stay
`no_std` compatible, while optional adapters provide std, transport, rendering, and
persistence behavior.

## Coding Agent Instructions

- **No emojis**: Do not use emojis in code or documentation. Remove them where they exist.
- **Git commits**: Commit small, focused, modular changes to the local git repo after implementing and testing each change.
- **Test efficiency**: Run tests for code changes only. Skip test runs for documentation-only changes.
- **Test performance**: Keep the primary test suite fast. Use `#[ignore]` for slow tests; property tests should use reasonable case counts.

## Architecture Principles

- **Library-first design**: Core logic lives in the library with an optional binary runner.
- **`no_std` compatibility**: Core/app/protocol/render/input traits should compile without std unless explicitly feature-gated.
- **Separation of concerns**: engine rules, app orchestration, agents, input, rendering, protocol, transport, and persistence are separate layers.
- **Adapter boundaries**: Runners compose `BattleshipApp` with agents, renderers, transports, and save stores.
- **Embedded-first**: Core mechanics should avoid heap-heavy designs and preserve fixed-size board data structures.

## Repository Structure

```text
battleship/
|-- src/
|   |-- lib.rs                 # Crate root and public API surface
|   |-- main.rs                # Thin std/CLI runner
|   |-- core/                  # Internal rules implementation
|   |-- agent/                 # PlayerAgent, AiAgent, HumanAgent, RemoteAgent, ScriptedAgent
|   |-- app/                   # BattleshipApp, MatchState, AppEvent, AppCommand
|   |-- input/                 # UiEvent and InputSource
|   |-- render/                # Passive ScreenView types and Renderer trait
|   |-- render_cli.rs          # CLI input/render adapters
|   |-- protocol/              # WireMessage and protocol domain types
|   |-- transport/             # TransportEndpoint plus std async transports
|   |-- persistence/           # SavedGame, adapter snapshots, SaveStore
|   `-- data_generation/       # Optional data-generation boundary
|-- tests/                     # Engine, app, protocol, transport, and adapter tests
|-- README.md
`-- overview.md
```

The public `engine` module re-exports the tracked core mechanics from `src/core`.

## Code Conventions

- Use domain-specific `Result<T, E>` types where they exist.
- Use `anyhow::Result` only at std/application boundaries such as transports and the CLI runner.
- Never panic in library code for recoverable errors.
- Keep rustdoc comments on public APIs.
- Prefer self-documenting code and use inline comments sparingly.
- Run `cargo fmt`, checks, tests, and clippy before completing code changes when the local toolchain supports them.

## Testing Requirements

- Engine and board logic belong in unit tests or focused integration/property tests.
- App orchestration belongs in tests using `BattleshipApp`, `AiAgent`, and `ScriptedAgent`.
- Protocol changes belong in serialization, version, and transport-resilience tests using `WireMessage`.
- Renderer changes should use passive `ScreenView` values and snapshot-style string assertions where useful.
- Slow or stress tests should be ignored or case-limited.

## Protocol & Networking

- Use `protocol::WireMessage` for wire semantics.
- Keep transports responsible for moving messages, not deciding game rules.
- Use `TransportEndpoint` for nonblocking app-facing polling and sending.
- Async TCP/WebSocket code should be runner-owned or wrapped behind adapters so app logic does not await I/O.
- Preserve version and sequence fields on game messages.

## Agents, Input, And Rendering

- Use `PlayerAgent` implementations for decision-making.
- Use `AiAgent` for probability-based AI with the tracked repo's current difficulty tuning.
- Use `HumanAgent` with `UiEvent` values for local human intent.
- Use `RemoteAgent` for protocol-observed remote actions.
- Use `ScriptedAgent` for deterministic tests.
- Renderers consume passive `ScreenView` values and must not own game flow.

## Persistence

- Use `SavedGame`, `AgentSnapshot`, `UiSnapshot`, and `AdapterState`.
- Use `SaveStore::{load_active, save_active, clear_active}` for app runners.
- Persistence must reference app/protocol snapshots rather than old player or interface state.

## Feature Gates

- `default = ["std", "cli", "ai", "tcp", "logging"]`
- `std`: standard-library support.
- `cli`: CLI runner and renderer.
- `ai`: AI agent behavior.
- `tcp`: TCP and heartbeat transports.
- `in-memory`: in-memory test transport.
- `persistence`: app-facing save snapshots and stores.
- `websocket`, `web`, `ble`, `esp-idf`, `data-generation`, `logging`: independent optional adapter boundaries.

## Common Commands

```bash
cargo fmt
cargo check
cargo check --no-default-features
cargo check --all-features
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-features -- -D warnings
```

If Windows clippy reports mixed GNU/MSVC artifacts, run clippy through a single rustup
toolchain or clear target artifacts before retrying.

## What To Avoid

- Reintroducing removed mixed player/interface or RPC helper APIs.
- Heap-heavy changes in core board, bitboard, ship, or game-rule paths.
- Direct game decisions inside transport or renderer code.
- Compatibility shims for removed legacy architecture unless explicitly requested.
- Committing untracked reference folders.
- Skipping `no_std` checks after touching core/app/protocol/render/input code.

## Documentation

- Keep `README.md` and `overview.md` aligned with the adapter architecture.
- Document protocol changes around `WireMessage`.
- Document future runner work as adapter composition around `BattleshipApp`.
