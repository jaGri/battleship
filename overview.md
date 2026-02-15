# Battleship Game Overview

Current snapshot of the Battleship codebase, its runtime flows, and suggested improvements.

## Repository Structure

```
battleship/
│
├── Cargo.toml                           # Rust package manifest; library-first design with optional binary and std feature
├── README.md                            # Project documentation and usage instructions
├── overview.md                          # Architecture overview, execution flows, and development roadmap
├── AGENTS.md                            # AI agent collaboration and task context
│
├── src/
│   ├── lib.rs                           # Crate root; re-exports public API surface
│   ├── main.rs                          # Binary entry point; CLI with local/tcp-server/tcp-client modes (std only)
│   │
│   ├── board.rs                         # Board state management using generic BitBoard for compact cell masks
│   ├── ship.rs                          # Ship placement logic and definitions
│   ├── bitboard.rs                      # Generic bitboard implementation for efficient set operations
│   ├── game.rs                          # GameEngine orchestrating own board, guesses, and remaining ships
│   ├── config.rs                        # Game configuration (10×10 board, ship sets, constants)
│   │
│   ├── player.rs                        # Player trait defining guess/status interface
│   ├── player_ai.rs                     # AI implementation using probability density and temperature sampling
│   ├── player_cli.rs                    # Human CLI player with input validation (std only)
│   ├── player_node.rs                   # PlayerNode wrapper orchestrating player + engine + transport
│   ├── ai.rs                            # AI heuristics: probability density over unguessed cells with hit bias
│   │
│   ├── protocol.rs                      # Wire protocol: versioned messages, handshake, sequence validation
│   ├── domain.rs                        # Domain types bridging game logic to protocol messages
│   ├── common.rs                        # Shared types and utilities
│   │
│   ├── cli.rs                           # CLI argument parsing and command structures (std only)
│   ├── interface_cli.rs                 # CLI rendering: board display with box-drawing, ship status
│   │
│   ├── skeleton.rs                      # Framework/scaffolding code for future extensions
│   ├── stub.rs                          # Placeholder implementations or stubs
│   │
│   └── transport/
│       ├── mod.rs                       # Transport trait and module exports
│       ├── in_memory.rs                 # In-memory channel transport for local AI vs AI games
│       ├── tcp.rs                       # Length-prefixed TCP transport with bincode framing, timeouts
│       └── heartbeat.rs                 # Heartbeat wrapper for active connection monitoring (10s interval, 45s timeout)
│
└── tests/
    ├── ai_game_tests.rs                 # AI vs AI game scenarios and strategies
    ├── ai_transport_game.rs             # AI games over transport layer
    ├── bitboard_tests.rs                # Bitboard operations and edge cases
    ├── board_props.rs                   # Property-based tests for board logic
    ├── board_tests.rs                   # Unit tests for board state
    ├── game_engine_props.rs             # Property-based tests for game engine
    ├── game_state_serialization.rs      # Serialization/deserialization validation
    │
    ├── cli_test.rs                      # CLI interface and rendering tests
    │
    ├── in_memory_transport_tests.rs     # In-memory transport validation
    ├── tcp_transport_tests.rs           # TCP transport connection and framing
    ├── tcp_game_test.rs                 # End-to-end TCP game scenarios
    ├── heartbeat_integration_test.rs    # Heartbeat monitoring for AI games
    │
    ├── protocol_hardening_tests.rs      # Handshake, version negotiation, timeout protection
    ├── player_node_robustness_tests.rs  # Version/sequence mismatch handling, unexpected messages
    ├── transport_resilience_tests.rs    # Graceful shutdown, bounded reads, error handling
    ├── sequence_tests.rs                # Strict sequence number validation
    ├── malformed_frame_tests.rs         # Invalid message format handling
    ├── fuzz_bincode_tests.rs            # Fuzzing tests for bincode deserialization
    ├── cross_version_tests.rs           # Protocol version compatibility tests
    ├── disconnect_reconnect_tests.rs    # Connection failure and recovery scenarios
    └── state_sync_tests.rs              # State synchronization message handling
```

### Key Architecture Points

**Core Modules:**
- **board.rs, ship.rs, bitboard.rs**: Game rules with compact bit-based state representation
- **game.rs**: Central `GameEngine` managing game state and win/loss detection
- **player.rs + implementations**: Trait-based player abstraction (AI, CLI)
- **player_node.rs**: Orchestrates full turn loop with transport

**Networking:**
- **protocol.rs**: Versioned wire protocol with handshake, sequence validation, heartbeats
- **transport/**: Abstract transport trait + in-memory and TCP implementations
- **transport/heartbeat.rs**: Connection health monitoring wrapper

**AI:**
- **ai.rs, player_ai.rs**: Probability-driven targeting with hit bias and temperature sampling

**Binary/CLI:**
- **main.rs, cli.rs, interface_cli.rs**: Optional binary with three execution modes (requires std feature)

**Testing:**
- Comprehensive test suite covering unit, property-based, integration, and protocol hardening scenarios

## Architecture and Modules

- **Crate setup**: Library-first design with optional binary; `no_std` compatible core gated by the `std` feature (default). Re-exports in [src/lib.rs](src/lib.rs) keep the public surface compact.
- **Core rules**: Board state and ship placement live in [src/board.rs](src/board.rs) and [src/ship.rs](src/ship.rs), using a generic `BitBoard` for compact masks. Game orchestration is in [src/game.rs](src/game.rs) with `GameEngine` tracking own board, guesses, and remaining enemy ships.
- **Players**: `Player` trait in [src/player.rs](src/player.rs); implementations include probability-driven AI in [src/player_ai.rs](src/player_ai.rs) and a CLI human player (compiled with `std`). `PlayerNode` in [src/player_node.rs](src/player_node.rs) wraps a player + engine + transport to drive a full turn loop.
- **AI heuristics**: [src/ai.rs](src/ai.rs) builds a probability density over unguessed cells, heavily biasing toward placements covering existing hits, then samples with temperature for exploration.
- **Protocol surface**: Domain-friendly types in [src/domain.rs](src/domain.rs) bridge to on-wire messages defined in [src/protocol.rs](src/protocol.rs). `GameApi` trait lets engines be served remotely.
- **Transport**: `Transport` trait plus in-memory channel ([src/transport/in_memory.rs](src/transport/in_memory.rs)) and length-prefixed TCP transport ([src/transport/tcp.rs](src/transport/tcp.rs)) using bincode framing.
- **Configuration**: [src/config.rs](src/config.rs) fixes board size (10×10) and ship set; helper `ship_name_static` normalizes names. `Cargo.toml` enables `std` feature to pull in tokio, async-trait, serde, bincode, clap, and `rand` std RNGs.

## Execution Flows

- **Binary CLI**: [src/main.rs](src/main.rs) exposes three commands via `clap` (only when `std` enabled):
	- `local`: AI vs AI using paired in-memory transports.
	- `tcp-server`: Hosts a listener, seeds either human CLI or AI player, serves over `TcpTransport`.
	- `tcp-client`: Connects to a server, seeds human CLI or AI player, then exchanges turns.
- **Turn loop**: `PlayerNode::run` (and the CLI helper in `main`) alternate between sending `Message::Guess` and responding with `Message::StatusResp`, advancing a sequence counter and stopping when `GameEngine::status` reports Won/Lost.

## Networking and Protocol

- **Messages**: Versioned envelope (`PROTOCOL_VERSION = 1`) with Handshake/HandshakeAck/Guess/StatusResp/Sync/Heartbeat and other variants ([src/protocol.rs](src/protocol.rs)). All messages carry version and sequence numbers for validation.
- **Handshake**: Before game start, players exchange `Handshake` and `HandshakeAck` messages to verify protocol compatibility. Version mismatches result in clear error messages and connection rejection.
- **Encoding**: `TcpTransport` uses a u32 big-endian length prefix + bincode payload with bounded read lengths (max 10MB) to prevent DoS attacks.
- **Timeouts**: Configurable timeout support (default 30 seconds) protects against network hangs. Created via `TcpTransport::with_timeout()` or uses defaults with `TcpTransport::new()`.
- **Sequence validation**: Strict sequence number tracking with separate counters for sending (`my_seq`) and receiving (`expected_recv_seq`). Out-of-order or duplicate messages are rejected immediately with detailed error logging.
- **Error handling**: All protocol violations (version mismatch, sequence errors, unexpected messages) trigger explicit errors and session closure. Logging via `eprintln!` provides detailed diagnostics including expected vs actual values.
- **State sync**: `SyncPayload` is currently an empty placeholder; `GameApi::sync_state` is a no-op in `GameEngine`.

## Testing Footprint

- Rich set of unit/property/integration tests under `tests/` covering:
  - Bitboards, board properties, AI games, in-memory and TCP transport, serialization
  - Protocol hardening: handshake version negotiation, sequence validation, timeout protection
  - Player node robustness: version/sequence mismatch handling, unexpected message rejection
  - Transport resilience: graceful shutdown, bounded message sizes, error mapping
- Uses `proptest` for property-based tests and tokio for async test cases
- Full end-to-end TCP game tests with AI players over local connections

## Recommendations and Next Steps

### High Priority Features

- **Authentication/Authorization** 🔐: Add authentication for network games to prevent unauthorized connections.
  - Implement shared secret or token-based auth
  - Add player identity verification
  - Consider rate limiting to prevent abuse

- **AI Evolution** 🤖: Enhance AI player with advanced targeting strategies.
  - Implement oriented hunt mode after finding a hit (chain hits into lines)
  - Add salvo/targeting modes for aggressive play
  - Explore adaptive temperature based on game state
  - Add benchmarks to track AI performance and prevent regressions

### Medium Priority Features

- **TLS/Encryption** 🔒: Add optional TLS support for encrypted TCP transport.
  - Integrate `tokio-rustls` for TLS support
  - Protect game data and credentials in transit
  - Make TLS optional via feature flag

- **Documentation & Examples** 📚: Expand project documentation for better onboarding.
  - Add comprehensive README with usage examples
  - Document network protocol specification
  - Explain AI strategy and probability calculations
  - Create deployment guide for server setup
  - Add architecture diagrams (mermaid flowcharts)

- **CI/CD Pipeline** 🔄: Automate testing and release process.
  - Set up GitHub Actions for automated testing
  - Add linting and format checks (clippy, rustfmt)
  - Implement release automation with semantic versioning
  - Add code coverage reporting

### Long-Term Enhancements

- **Benchmarking Suite** ⚡: Track and optimize performance over time.
  - Add criterion.rs benchmarks for AI decision-making
  - Benchmark serialization/deserialization performance
  - Track bitboard operations performance

- **Advanced Features** ✨: Extend gameplay and observability.
  - Add spectator mode for watching games in progress
  - Implement game replay functionality
  - Add telemetry/metrics for game analysis
  - Create web-based UI for browser play
  - Support tournament mode with multiple concurrent games

- **Platform Expansion** 🌐: Broaden deployment options.
  - WebAssembly (WASM) compilation for browser support
  - Mobile-friendly protocol (consider BLE transport completion)
  - Cross-platform GUI using egui or iced

## Completed Features

- ✅ **Protocol hardening**: Handshake with version negotiation, configurable timeouts (default 30s), strict sequence validation with rejection of out-of-order/duplicate messages
- ✅ **Transport resilience**: Bounded read lengths (max 10MB), graceful error handling and session closure, timeout protection on all network operations
- ✅ **Player loop robustness**: Explicit handling of unexpected messages with session closure, detailed logging of version/sequence mismatches for debugging
- ✅ **TCP transport**: Full implementation with `TcpTransport::connect()` and `TcpTransport::new()` supporting both client and server modes
- ✅ **CLI interface**: Three-mode operation via clap (local/tcp-server/tcp-client) with configurable bind addresses and connection endpoints
- ✅ **CLI/UX polish**: Enhanced board rendering with box-drawing characters and ship status; comprehensive input validation with bounds checking and duplicate guess detection; contextual help text for placement and targeting; RNG seed flag (--seed) for reproducible games across all commands
- ✅ **Active heartbeat monitoring**: Periodic heartbeat messages with idle connection detection (10s interval, 45s timeout), automatic heartbeat echo, transparent filtering from game logic, graceful connection closure on timeout
