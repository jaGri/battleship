# Agent Guidelines for Battleship

Guidelines for AI agents working on this Rust Battleship implementation.

## Architecture Principles

- **Library-first design**: Core logic in `src/lib.rs` with optional binary. Keep public API minimal.
- **`no_std` compatibility**: Core modules (ai, bitboard, board, game, ship) must work without std. Gate std-dependent features (serde, tokio, clap) behind `std` feature flag.
- **Zero heap allocations**: Use stack-based data structures. BitBoards and fixed-size arrays only.
- **Separation of concerns**: game logic → protocol → transport. Never mix layers.

## Code Conventions

### Error Handling
- Use `Result<T, E>` with domain-specific errors (`BoardError`, `BitBoardError`)
- Implement `std::error::Error` for all error types (gated by `#[cfg(feature = "std")]`)
- Use `anyhow::Result` only at application boundaries (transport, CLI)
- Never panic in library code; return errors

### Types & Generics
- BitBoard is generic: `BitBoard<T, const N: usize>` where `T: PrimInt + Unsigned`
- Board size is const: use `BOARD_SIZE`, `NUM_SHIPS`, `SHIPS` from config
- Prefer `&'static str` for ship names (compile-time constants)

### Async & Traits
- All transport and protocol operations are async (tokio runtime)
- Use `#[async_trait]` for async trait methods
- Transport trait must be `Send + Sync` for concurrent games

## Testing Requirements

### Property-Based Tests (proptest)
- Add property tests for new game logic in `tests/game_engine_props.rs`
- Use `proptest!` macro with reasonable case counts (100-500)
- Test round-trip serialization for all protocol types

### Integration Tests
- Protocol changes require tests in `tests/protocol_hardening_tests.rs`
- Network features need TCP transport tests
- AI changes should update `tests/ai_game_tests.rs`

### Test Organization
- Unit tests: inline `#[cfg(test)]` modules
- Integration tests: `tests/*.rs` files
- Proptest regressions: ignored via `.gitignore`

## Protocol & Networking

### Message Design
- All messages carry `version` and `seq` (except Handshake/HandshakeAck/Heartbeat)
- Use `PROTOCOL_VERSION` constant; increment on breaking changes
- Handshake before any game messages; validate version compatibility

### Transport Constraints
- Max message size: 10MB (`MAX_MESSAGE_SIZE`)
- Default timeout: 30s (`DEFAULT_TIMEOUT`)
- Use length-prefixed bincode framing (u32 big-endian length + payload)
- Always bound reads to prevent DoS

### Error Handling in Network Code
- Log protocol violations with `eprintln!` (include expected vs actual)
- Close session on version/sequence mismatches
- Use `anyhow::Context` for detailed error messages

## AI Development

- Probability maps are `[[f32; GRID_SIZE]; GRID_SIZE]`
- Temperature sampling for exploration (avoid greedy selection)
- Future work: implement hunt mode after hits (chain detection)
- Add benchmarks before optimizing AI logic

## Common Patterns

### BitBoard Operations
```rust
let mut bb = BitBoard::<u128, 10>::new();
bb.set(row, col)?;  // fallible
if bb.get(row, col)? { ... }
```

### Ship Placement
```rust
let (r, c, o) = engine.board_mut().random_placement(&mut rng, ship_idx)?;
engine.board_mut().place(ship_idx, r, c, o)?;
```

### Protocol Messages
```rust
Message::Guess { version: PROTOCOL_VERSION, seq: my_seq, x, y }
```

## What to Avoid

- ❌ Heap allocations in core game logic
- ❌ Unwrapping without checking (use `?` or `unwrap_or`)
- ❌ Breaking `no_std` compatibility in core modules
- ❌ Direct TCP operations (use Transport trait)
- ❌ Skipping git hooks or forcing destructive operations
- ❌ Committing `*.proptest-regressions` files

## Documentation

- Add doc comments (`///`) for public APIs
- Keep `overview.md` updated with architecture changes
- Update `Completed Features` section when implementing roadmap items
- Use inline comments sparingly; prefer self-documenting code

## Performance

- Bitboard operations are hot paths; avoid allocations
- AI probability calculation is O(n²); keep grid traversals tight
- Bincode serialization is fast; don't optimize prematurely
- Add benchmarks (criterion) before performance work
