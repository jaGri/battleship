# Agent Guidelines for Battleship

Guidelines for AI agents working on this Rust Battleship implementation.

## Project Context

This is a Rust implementation of Battleship designed for deployment on ESP32 embedded devices with physical controls and LCD displays. The project prioritizes testing various player implementations, interfaces, and transport layers while maintaining `no_std` compatibility for embedded targets.

## Coding Agent Instructions

- **No emojis**: Do not use emojis in code or documentation. Remove them where they exist.
- **Git commits**: Commit small, focused, modular changes to the local git repo after implementing and testing each change.
- **Test efficiency**: Run tests for code changes only. Skip test runs for documentation-only changes.
- **Test performance**: Keep the primary test suite fast. Use `#[ignore]` for slow tests; property tests should use reasonable case counts (100-500).

## Architecture Principles

- **Library-first design**: Core logic in `src/lib.rs` with optional binary. Keep public API minimal and well-documented.
- **`no_std` compatibility**: Core modules (ai, bitboard, board, game, ship) must work without std. Gate std-dependent features (serde, tokio, clap, rand std RNGs) behind `std` feature flag.
- **Zero heap allocations**: Use stack-based data structures. BitBoards and fixed-size arrays only in core game logic.
- **Separation of concerns**: game logic → protocol → transport. Never mix layers.
- **Embedded-first**: Design decisions should consider ESP32 constraints (~256KB RAM, no heap allocator in core paths).

## Repository Structure

```
battleship/
├── src/
│   ├── lib.rs                    # Crate root; minimal public API surface
│   ├── main.rs                   # Binary entry point (CLI modes, std only)
│   │
│   ├── board.rs                  # Board state with BitBoard
│   ├── ship.rs                   # Ship placement logic
│   ├── bitboard.rs               # Generic bitboard implementation
│   ├── game.rs                   # GameEngine orchestration
│   ├── config.rs                 # Game configuration (10×10, ship sets)
│   │
│   ├── player.rs                 # Player trait
│   ├── player_ai.rs              # AI with probability density
│   ├── player_cli.rs             # Human CLI player (std only)
│   ├── player_node.rs            # PlayerNode orchestrator
│   ├── ai.rs                     # AI heuristics
│   │
│   ├── protocol.rs               # Wire protocol with versioning
│   ├── domain.rs                 # Domain types for protocol
│   ├── common.rs                 # Shared utilities
│   │
│   ├── cli.rs                    # CLI argument parsing (std only)
│   ├── interface_cli.rs          # CLI rendering with box-drawing
│   │
│   └── transport/
│       ├── mod.rs                # Transport trait
│       ├── in_memory.rs          # In-memory channels
│       ├── tcp.rs                # TCP with bincode framing
│       └── heartbeat.rs          # Connection monitoring wrapper
│
└── tests/                        # Comprehensive test suite
```

## Code Conventions

### Error Handling
- Use `Result<T, E>` with domain-specific errors (`BoardError`, `BitBoardError`, `TransportError`)
- Implement `std::error::Error` for all error types (gated by `#[cfg(feature = "std")]`)
- Use `anyhow::Result` only at application boundaries (transport, CLI, main.rs)
- Never panic in library code; return errors with context
- Log protocol violations with `eprintln!` including expected vs actual values

### Types & Generics
- BitBoard is generic: `BitBoard<T, const N: usize>` where `T: PrimInt + Unsigned`
- Board size is const: use `BOARD_SIZE`, `NUM_SHIPS`, `SHIPS` from config
- Prefer `&'static str` for ship names (compile-time constants)
- Use const generics where possible to enable compile-time validation

### Async & Traits
- All transport and protocol operations are async (tokio runtime when `std` enabled)
- Use `#[async_trait]` for async trait methods
- Transport trait must be `Send + Sync` for concurrent games
- Prefer bounded channels to prevent unbounded memory growth

### Code Style
- Run `cargo fmt` and `cargo clippy -- -D warnings` before completing tasks
- All public APIs must have rustdoc comments (`///`)
- Use inline comments sparingly; prefer self-documenting code
- Error messages should be descriptive with context for debugging

## Testing Requirements

### Test Organization
- **Unit tests**: Inline `#[cfg(test)]` modules for local validation
- **Integration tests**: `tests/*.rs` files for cross-module scenarios
- **Property tests**: `tests/*_props.rs` files using proptest
- **Proptest regressions**: Automatically ignored via `.gitignore`

### Property-Based Tests (proptest)
- Add property tests for new game logic in `tests/game_engine_props.rs` and `tests/board_props.rs`
- Use `proptest!` macro with reasonable case counts (100-500 for fast suite)
- Test round-trip serialization for all protocol types in `tests/game_state_serialization.rs`
- Mark expensive property tests with `#[ignore]` for optional deep testing

### Integration & Protocol Tests
- Protocol changes require tests in `tests/protocol_hardening_tests.rs`
- Handshake and version negotiation tests in `tests/cross_version_tests.rs`
- Network features need tests in `tests/tcp_transport_tests.rs` and `tests/tcp_game_test.rs`
- AI behavior changes should update `tests/ai_game_tests.rs`
- Transport resilience tests cover graceful shutdown, bounded reads, error handling

### Test Performance
- Primary test suite should complete in under 30 seconds
- Use `cargo test --release` for performance-sensitive tests
- Mark slow tests (>5s) with `#[ignore]` and document why
- Use `InMemoryTransport` for tests instead of TCP when possible (faster, deterministic)

## Protocol & Networking

### Message Design
- All messages carry `version` and `seq` fields (except Handshake/HandshakeAck/Heartbeat)
- Use `PROTOCOL_VERSION` constant; increment on breaking changes
- Handshake must complete before any game messages; validate version compatibility
- Sequence numbers are strictly validated: separate counters for send (`my_seq`) and receive (`expected_recv_seq`)
- Out-of-order or duplicate messages are rejected immediately with detailed error logging

### Transport Constraints
- Max message size: 10MB (`MAX_MESSAGE_SIZE`) to prevent DoS attacks
- Default timeout: 30s (`DEFAULT_TIMEOUT`), configurable via `with_timeout()`
- Use length-prefixed bincode framing (u32 big-endian length + payload)
- Always bound reads with `take()` to prevent unbounded memory consumption
- Close session on protocol violations (version mismatch, sequence errors, unexpected messages)

### Heartbeat Monitoring
- Wrap `TcpTransport` with `HeartbeatTransport` for connection health monitoring
- Use `HeartbeatTransport::disabled()` wrapper for `InMemoryTransport` (no heartbeat needed)
- Default intervals: 10s heartbeat, 45s idle timeout (`HEARTBEAT_INTERVAL`, `IDLE_TIMEOUT`)
- Heartbeats are transparent (filtered before returning to game logic)
- `tokio::select!` multiplexes message receiving with periodic heartbeat sending
- Automatic heartbeat echo and graceful connection closure on timeout

### Error Handling in Network Code
- Log protocol violations with context: "Expected seq X, got Y from version Z"
- Use `anyhow::Context` for detailed error messages in transport layer
- Map transport errors to appropriate protocol errors
- Return clear error messages for handshake failures, timeouts, and disconnects

## AI Player

### Current Implementation
- Probability maps are `[[f32; GRID_SIZE]; GRID_SIZE]` computed per guess
- Heavily bias probability toward cells that could contain ships overlapping existing hits
- Temperature sampling for exploration (avoid purely greedy selection)
- Supports configurable temperature via `AIPlayer::with_temperature()`

### Future Work
- Implement oriented hunt mode after hits (chain detection along cardinal directions)
- Add salvo/targeting modes for aggressive play strategies
- Explore adaptive temperature based on game state (hit streaks, remaining ships)
- Add benchmarks using criterion before optimizing AI logic

## Common Patterns

### BitBoard Operations
```rust
let mut bb = BitBoard::<u128, 10>::new();
bb.set(row, col)?;  // Fallible operations return Result
if bb.get(row, col)? {
    // Cell is set
}
let count = bb.count();  // Use bitwise operations for efficiency
```

### Ship Placement
```rust
let (r, c, o) = engine.board_mut().random_placement(&mut rng, ship_idx)?;
engine.board_mut().place(ship_idx, r, c, o)?;
```

### Protocol Messages
```rust
// Messages with sequence tracking
Message::Guess {
    version: PROTOCOL_VERSION,
    seq: my_seq,
    x,
    y,
}

// Handshake before game start
Message::Handshake { version: PROTOCOL_VERSION }
```

### Transport Usage
```rust
// TCP with timeout
let transport = TcpTransport::connect(&addr).await?;
let transport = HeartbeatTransport::wrap(transport);

// In-memory for testing
let (tx, rx) = in_memory_transport_pair();
```

## What to Avoid

- **Heap allocations** in core game logic (board, bitboard, game, ai, ship modules)
- **Unwrapping** without checking: use `?` operator or `unwrap_or()` with justification
- **Breaking `no_std`** compatibility in core modules (test with `cargo test --no-default-features`)
- **Direct TCP operations**: always use Transport trait abstraction
- **Skipping git hooks** or forcing destructive operations
- **Committing** `*.proptest-regressions` files (should be gitignored)
- **Large commits**: prefer small, focused changes with clear commit messages
- **Emojis**: anywhere in code or documentation

## Documentation

- Add rustdoc comments (`///`) for all public APIs with examples where helpful
- Keep `overview.md` updated with architecture changes and completed features
- Update "Completed Features" section when implementing roadmap items
- Protocol changes should be documented in future `docs/PROTOCOL.md`
- Embedded deployment instructions should eventually live in `docs/DEPLOYMENT.md`

## Performance

### Hot Paths
- Bitboard operations are performance-critical; avoid allocations and prefer bitwise ops
- AI probability calculation is O(n²) per guess; keep grid traversals tight
- Bincode serialization is already fast; don't optimize prematurely

### Benchmarking
- Add criterion benchmarks before performance optimization work
- Benchmark AI decision-making, serialization, and bitboard operations separately
- Document performance requirements for embedded targets (ESP32)

## Feature Gates

### Feature Gate Requirements

**Core principle**: Everything not essential to the base game engine must be feature-gated.

### Mandatory Feature Gates
- **std**: Anything using heap, async, I/O, networking
- **cli**: Terminal rendering, human input (`player_cli.rs`, `interface_cli.rs`)
- **tcp**: TCP transport and heartbeat monitoring
- **in-memory**: Channel-based transport (testing only)

### Current Features
- `std` (default): Enables stdlib, async (tokio), serde, bincode, clap, rand std RNGs
- Core game logic works in `no_std` mode
- All features that do not apply to all game modes (players, interfaces, transports, etc) should be strictly feature gated. 

### Future Features
- `bluetooth`: Bluetooth transport for ESP32
- `tls`: Optional TLS support for encrypted TCP
- `lcd`: LCD display interface for embedded UI

### Testing Feature Combinations
```bash
cargo test --all-features              # Full feature set
cargo test --no-default-features       # no_std compatibility
cargo test --features std              # Explicit std only
```

## Common Tasks

### Adding a New Transport
1. Create `src/transport/your_transport.rs`
2. Implement the `Transport` trait (async send/receive)
3. Add feature gate in `Cargo.toml` if needed
4. Write integration tests in `tests/your_transport_tests.rs`
5. Update `src/transport/mod.rs` exports
6. Consider heartbeat wrapper compatibility
7. Document usage examples in rustdoc

### Protocol Changes
1. Update message definitions in `src/protocol.rs`
2. Increment `PROTOCOL_VERSION` if breaking backward compatibility
3. Add serialization round-trip tests in `tests/game_state_serialization.rs`
4. Add handshake tests for version validation in `tests/cross_version_tests.rs`
5. Update protocol documentation when `docs/PROTOCOL.md` exists

### Adding AI Strategies
1. Modify `src/ai.rs` for new probability calculations
2. Update `src/player_ai.rs` if player behavior changes
3. Add test scenarios in `tests/ai_game_tests.rs`
4. Consider adding benchmarks if performance-critical
5. Document strategy in rustdoc comments

## Validation Checklist

After making changes, run:
```bash
cargo fmt                                    # Format code
cargo clippy -- -D warnings                  # Lint with warnings as errors
cargo test --all-features                    # Full test suite
cargo test --no-default-features             # Verify no_std compatibility
cargo build --target thumbv7em-none-eabihf   # Test embedded target (if available)
```

For property tests and integration tests:
```bash
cargo test --release                         # Faster execution for expensive tests
cargo test -- --ignored                      # Run slow/ignored tests
```

## Known Issues & Gotchas

- **Sequence numbers**: Protocol has strict sequence validation; don't skip or reorder messages
- **Timeouts**: Default is 30s, configurable via `TcpTransport::with_timeout()`
- **Heartbeats**: 10s interval, 45s idle timeout - don't make them too aggressive or connections will drop
- **Bitboards**: Generic over storage type; changes need careful consideration for embedded constraints
- **Bincode frame size**: Capped at 10MB to prevent DoS; don't increase without security review
- **ESP32 RAM**: ~256KB available; keep allocations minimal and stack-based

## Questions or Uncertainty

If you're unsure about:
- **Architecture decisions**: Propose options with trade-offs and wait for human input
- **Breaking changes**: Document impact and create a migration plan
- **Performance trade-offs**: Add benchmarks and present data before optimization
- **Embedded constraints**: Verify memory/CPU impact, test on target if possible