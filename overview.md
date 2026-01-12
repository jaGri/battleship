# Battleship Game Overview

Current snapshot of the Battleship codebase, its runtime flows, and suggested improvements.

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

- **State sync**: Implement `SyncPayload` and a real `GameApi::sync_state` to support reconnection/resume and to fix `enemy_ships_remaining` reconstruction (currently reset to all `true` in `GameEngine::from_state`).
- **Authentication**: Consider adding auth or shared secrets for network games to prevent unauthorized connections.
- **AI evolution**: Explore adaptive temperature, salvo/targeting modes, and heuristics that chain hits into oriented hunts; add benchmarks to avoid regressions.
- **Keepalive/heartbeat**: Implement active heartbeat messages for idle connection detection (message type exists but not yet used).
- **Additional tests**: Integration tests for disconnect/reconnect scenarios, malformed frames, and cross-version play; property tests for `GameEngine::record_guess`/`from_state` round-tripping; fuzz bincode frames.
- **TLS/encryption**: Add optional TLS support for encrypted TCP transport to protect game data in transit.

## Completed Features

- ✅ **Protocol hardening**: Handshake with version negotiation, configurable timeouts (default 30s), strict sequence validation with rejection of out-of-order/duplicate messages
- ✅ **Transport resilience**: Bounded read lengths (max 10MB), graceful error handling and session closure, timeout protection on all network operations
- ✅ **Player loop robustness**: Explicit handling of unexpected messages with session closure, detailed logging of version/sequence mismatches for debugging
- ✅ **TCP transport**: Full implementation with `TcpTransport::connect()` and `TcpTransport::new()` supporting both client and server modes
- ✅ **CLI interface**: Three-mode operation via clap (local/tcp-server/tcp-client) with configurable bind addresses and connection endpoints
- ✅ **CLI/UX polish**: Enhanced board rendering with box-drawing characters and ship status; comprehensive input validation with bounds checking and duplicate guess detection; contextual help text for placement and targeting; RNG seed flag (--seed) for reproducible games across all commands
