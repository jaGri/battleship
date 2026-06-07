# Battleship Repository Reorganization Plan

**Date:** 2026-02-15
**Status:** Planning Phase
**Goal:** Reorganize the repository for better modularity, maintainability, and drop-in component replacement

---

## Executive Summary

This document outlines a comprehensive reorganization of the battleship repository from a flat `src/` directory structure to a **layered architecture** that clearly separates concerns:

1. **Core game engine** (no_std compatible)
2. **Player interfaces** (AI, CLI, future: web, hardware)
3. **Transports** (memory, TCP, future: Bluetooth, WebSocket)
4. **Protocol layer** (network messaging, RPC)
5. **Application layer** (CLI binary)

The reorganization preserves all existing functionality while enabling true drop-in replacement of components and clearer architectural boundaries.

---

## Current State Analysis

### Directory Structure

Currently, the repository has a **flat structure** with 19 files in `src/`:

```
src/
├── ai.rs                    (184 lines) AI probability calculations
├── bitboard.rs              (366 lines) Generic bitboard implementation
├── board.rs                 (217 lines) Board state management
├── cli.rs                   (32 lines) CLI command definitions
├── common.rs                (66 lines) GuessResult, errors
├── config.rs                (25 lines) Game constants
├── domain.rs                (59 lines) Serializable types
├── game.rs                  (200 lines) GameEngine
├── interface_cli.rs         (36 lines) Display helpers
├── lib.rs                   (112 lines) Root library
├── main.rs                  (407 lines) CLI binary
├── player.rs                (31 lines) Player trait
├── player_ai.rs             (45 lines) AI player
├── player_cli.rs            (375 lines) CLI player
├── player_node.rs           (272 lines) Player orchestrator
├── protocol.rs              (76 lines) Message enum
├── ship.rs                  (200 lines) Ship placement
├── skeleton.rs              (141 lines) Server RPC
├── stub.rs                  (105 lines) Client RPC
└── transport/
    ├── mod.rs               (16 lines) Transport trait
    ├── in_memory.rs         (153 lines) In-memory transport
    ├── tcp.rs               (241 lines) TCP transport
    └── heartbeat.rs         (178 lines) Heartbeat wrapper
```

**Total:** ~3,500 lines across 23 files

### Current Strengths

The codebase already has excellent architectural patterns:

✅ **Trait-based abstractions**: `Player`, `Transport`, `GameApi` traits enable polymorphism
✅ **no_std compatibility**: Core game logic (bitboard, board, ship, game, ai) works without std
✅ **Feature gating**: `#[cfg(feature = "std")]` properly separates std-dependent code
✅ **Zero-allocation core**: Fixed-size arrays, stack-based storage for embedded systems
✅ **Protocol versioning**: Version negotiation and sequence tracking
✅ **Comprehensive testing**: 21+ test files covering all layers

### Current Pain Points

❌ **Flat structure**: Hard to distinguish core logic from infrastructure
❌ **Unclear boundaries**: Which files are no_std vs std-only?
❌ **Difficult navigation**: 19 files at the same level, hard to find related code
❌ **Extension friction**: Where should `web.rs` or `bluetooth.rs` go?
❌ **Dependency flow unclear**: Which components depend on which?

---

## Proposed Structure: Layered Architecture

### New Directory Layout

```
src/
├── lib.rs                          # Public API with layer-based re-exports
├── main.rs                         # Minimal CLI entry point (calls cli::run())
│
├── core/                           # Layer 1: Game engine (no_std)
│   ├── mod.rs                      # Module orchestration, re-exports
│   ├── bitboard.rs                 # Generic bitboard (366 lines)
│   ├── board.rs                    # Board state & ship placement (217 lines)
│   ├── ship.rs                     # Ship types & placement logic (200 lines)
│   ├── game.rs                     # GameEngine orchestrator (200 lines)
│   ├── ai.rs                       # AI probability calculations (184 lines)
│   ├── config.rs                   # Game constants (25 lines)
│   └── common.rs                   # GuessResult, errors (66 lines)
│
├── player/                         # Layer 2: Player abstractions
│   ├── mod.rs                      # Player trait definition
│   ├── ai.rs                       # AiPlayer (from player_ai.rs, 45 lines)
│   ├── cli.rs                      # CliPlayer (from player_cli.rs, 375 lines)
│   └── node.rs                     # PlayerNode orchestrator (from player_node.rs, 272 lines)
│
├── transport/                      # Layer 2: Transport abstractions
│   ├── mod.rs                      # Transport trait (16 lines)
│   ├── in_memory.rs                # Paired channels (153 lines)
│   ├── tcp.rs                      # TCP networking (241 lines)
│   └── heartbeat.rs                # Connection health wrapper (178 lines)
│
├── protocol/                       # Layer 3: Network protocol & RPC
│   ├── mod.rs                      # Message enum, GameApi, version (from protocol.rs, 76 lines)
│   ├── domain.rs                   # Serializable domain types (59 lines)
│   ├── skeleton.rs                 # Server-side RPC handler (141 lines)
│   └── stub.rs                     # Client-side RPC proxy (105 lines)
│
└── cli/                            # Layer 4: Application interface
    ├── mod.rs                      # CLI struct, command definitions (from cli.rs, 32 lines)
    ├── commands.rs                 # Command handlers (extracted from main.rs, ~350 lines)
    └── interface.rs                # Display helpers (from interface_cli.rs, 36 lines)
```

### Architectural Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Application (cli/)                                 │
│ - User-facing CLI                                           │
│ - Command parsing and execution                             │
└─────────────────────────────────────────────────────────────┘
                          ↑
                          │
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: Protocol (protocol/)                               │
│ - Network message definitions                               │
│ - RPC framework (Skeleton/Stub)                             │
│ - Protocol versioning                                       │
└─────────────────────────────────────────────────────────────┘
                          ↑
                          │
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: Abstractions (player/, transport/)                 │
│ - Player trait + implementations (AI, CLI)                  │
│ - Transport trait + implementations (Memory, TCP)           │
│ - PlayerNode orchestrator                                   │
└─────────────────────────────────────────────────────────────┘
                          ↑
                          │
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Core (core/)                                       │
│ - Pure game logic (no_std compatible)                       │
│ - Board, Ship, GameEngine, BitBoard                         │
│ - AI probability calculations                               │
└─────────────────────────────────────────────────────────────┘
```

**Dependency Rule:** Code can only depend on layers below it, never above or sideways.

---

## Design Rationale

### Why Layered Architecture?

We considered three approaches:

1. **Layered Architecture** (Recommended)
2. Feature-Oriented Architecture (engine/, ai/, human/, network/)
3. Hexagonal/Ports & Adapters (domain/, ports/, adapters/)

**Layered architecture wins because:**

✅ **Clear dependency hierarchy**: Layers naturally flow from core → player/transport → protocol → cli
✅ **Matches current design**: The codebase already has implicit layers via feature gates
✅ **Easy drop-in replacement**: `player/web.rs` or `transport/bluetooth.rs` are single files
✅ **Preserves no_std boundary**: `core/` module clearly delineated
✅ **Balanced granularity**: 5 main modules, not too flat or nested
✅ **Rust conventions**: Aligns with common Rust project structures

### Feature-Oriented Architecture (Rejected)

**Structure would be:**
- `engine/` - Core game logic
- `ai/` - AI player
- `human/` - Human interfaces
- `network/` - Networking

**Why rejected:**
- Less clear dependency hierarchy
- Player trait definition location ambiguous (in engine? separate?)
- Mixes abstraction levels
- "Human" is unclear (CLI? Web? Hardware?)

### Hexagonal/Ports & Adapters (Rejected)

**Structure would be:**
- `domain/` - Core domain logic
- `ports/` - Abstract interfaces (traits)
- `adapters/` - Concrete implementations

**Why rejected:**
- Over-engineered for a ~3,500 line codebase
- More directories to navigate
- Unfamiliar terminology for Rust community
- Benefits don't outweigh added complexity

---

## Module Responsibilities

### Layer 1: core/ (no_std compatible)

**Purpose:** Pure game logic with zero external dependencies (except `num-traits`, `libm`, `rand::alloc`)

**Files:**
- `mod.rs` - Module declaration, re-exports common types
- `bitboard.rs` - Generic bitboard using const generics
- `board.rs` - Board state, ship placement validation
- `ship.rs` - Ship definitions, orientation, placement logic
- `game.rs` - GameEngine, turn management, game state
- `ai.rs` - Probability density functions, AI heuristics
- `config.rs` - Constants (BOARD_SIZE, SHIPS array)
- `common.rs` - GuessResult, BoardError, domain errors

**Public API:**
```rust
pub use core::{
    BitBoard, Board, GameEngine, Ship, GuessResult,
    calc_pdf, choose_best_square, // AI utilities
};
```

**Feature gates:** None (always available, even in no_std)

---

### Layer 2a: player/ (depends on core)

**Purpose:** Player trait and concrete implementations

**Files:**
- `mod.rs` - Player trait definition
  ```rust
  pub trait Player: Send {
      fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError>;
      fn select_target(&mut self, ...) -> (usize, usize);
      fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult);
      fn handle_opponent_guess(&mut self, coord: (usize, usize), result: GuessResult);
  }
  ```
- `ai.rs` - AiPlayer (uses `core::ai` for calculations)
- `cli.rs` - CliPlayer (interactive CLI, requires std)
- `node.rs` - PlayerNode (orchestrates Player + GameEngine + Transport)

**Public API:**
```rust
pub use player::{Player, AiPlayer};
#[cfg(feature = "std")]
pub use player::{CliPlayer, PlayerNode};
```

**Feature gates:**
- `Player` trait: no_std compatible (no feature gate)
- `AiPlayer`: no_std compatible
- `CliPlayer`, `PlayerNode`: require `#[cfg(feature = "std")]`

**Extension pattern:**
```rust
// Future: src/player/web.rs
#[cfg(all(feature = "std", target_arch = "wasm32"))]
pub struct WebPlayer { /* ... */ }
impl Player for WebPlayer { /* ... */ }
```

---

### Layer 2b: transport/ (depends on core for domain types)

**Purpose:** Transport trait and concrete implementations

**Files:**
- `mod.rs` - Transport trait
  ```rust
  #[async_trait]
  pub trait Transport: Send + Sync {
      async fn send(&mut self, msg: Message) -> anyhow::Result<()>;
      async fn recv(&mut self) -> anyhow::Result<Message>;
  }
  ```
- `in_memory.rs` - InMemoryTransport (paired channels for local games)
- `tcp.rs` - TcpTransport (length-prefixed bincode framing)
- `heartbeat.rs` - HeartbeatTransport (wrapper for connection monitoring)

**Public API:**
```rust
#[cfg(feature = "std")]
pub use transport::{Transport, TcpTransport, HeartbeatTransport};
```

**Feature gates:** All require `#[cfg(feature = "std")]` (async, serde, bincode)

**Extension pattern:**
```rust
// Future: src/transport/bluetooth.rs
#[cfg(feature = "bluetooth")]
pub struct BluetoothTransport { /* ... */ }
#[async_trait]
impl Transport for BluetoothTransport { /* ... */ }
```

---

### Layer 3: protocol/ (depends on core, player, transport)

**Purpose:** Network protocol definitions and RPC framework

**Files:**
- `mod.rs` - Message enum, GameApi trait, PROTOCOL_VERSION
  ```rust
  pub const PROTOCOL_VERSION: u8 = 1;

  pub enum Message {
      Handshake { version: u8 },
      Guess { seq: u32, coord: (usize, usize) },
      StatusResp { seq: u32, result: GuessResult },
      // ...
  }

  #[async_trait]
  pub trait GameApi: Send + Sync {
      async fn place_ships(&mut self, ships: Vec<Ship>) -> Result<(), String>;
      async fn guess(&mut self, coord: (usize, usize)) -> Result<GuessResult, String>;
      // ...
  }
  ```
- `domain.rs` - Serializable domain types (DomainBoard, DomainShip)
- `skeleton.rs` - Skeleton (server-side RPC handler)
- `stub.rs` - Stub (client-side RPC proxy)

**Public API:**
```rust
#[cfg(feature = "std")]
pub use protocol::{Message, GameApi, PROTOCOL_VERSION, Skeleton, Stub};
```

**Feature gates:** All require `#[cfg(feature = "std")]`

---

### Layer 4: cli/ (depends on all layers)

**Purpose:** Command-line application interface

**Files:**
- `mod.rs` - CLI struct, command definitions (using `clap`)
  ```rust
  #[derive(Parser)]
  pub struct Cli {
      #[command(subcommand)]
      command: Commands,
  }

  pub enum Commands {
      Local { player1: PlayerType, player2: PlayerType },
      TcpServer { port: u16, player: PlayerType },
      TcpClient { addr: SocketAddr, player: PlayerType },
  }

  pub async fn run() -> anyhow::Result<()> {
      // Entry point for CLI
  }
  ```
- `commands.rs` - Command handlers (extracted from main.rs)
- `interface.rs` - Display helpers (board visualization)

**Public API:**
```rust
#[cfg(feature = "std")]
pub use cli::run; // Entry point for binary
```

**Feature gates:** All require `#[cfg(feature = "std")]`

---

## Public API Design

### Root Library (lib.rs)

The root library re-exports commonly used types for convenience while maintaining module organization:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

// ============================================================================
// Layer 1: Core game engine (no_std compatible)
// ============================================================================
pub mod core;

// Convenience re-exports
pub use core::{
    // Types
    BitBoard, BitBoardError,
    Board, BoardState,
    Ship, ShipDef, ShipState, Orientation,
    GameEngine, GameState, GameStatus, GuessBoardState,
    GuessResult, BoardError,

    // Constants
    BOARD_SIZE, NUM_SHIPS, SHIPS,

    // AI functions
    calc_pdf, choose_best_square,
};

// ============================================================================
// Layer 2: Player & Transport
// ============================================================================
pub mod player;
pub use player::{Player, AiPlayer};

#[cfg(feature = "std")]
pub use player::{CliPlayer, PlayerNode};

#[cfg(feature = "std")]
pub mod transport;

#[cfg(feature = "std")]
pub use transport::{
    Transport,
    TcpTransport,
    HeartbeatTransport,
    // InMemoryTransport is internal, not re-exported
};

// ============================================================================
// Layer 3: Protocol & RPC
// ============================================================================
#[cfg(feature = "std")]
pub mod protocol;

#[cfg(feature = "std")]
pub use protocol::{
    Message,
    GameApi,
    PROTOCOL_VERSION,
    Skeleton,
    Stub,
    domain, // Module re-export for DomainBoard, etc.
};

// ============================================================================
// Layer 4: CLI (not exported by default, used by binary)
// ============================================================================
#[cfg(feature = "std")]
pub mod cli;
```

### Backward Compatibility

Users importing `use battleship::*;` continue to get the same types:
- `Board`, `GameEngine`, `Ship`, `GuessResult`
- `Player`, `AiPlayer`, `CliPlayer`
- `Transport`, `TcpTransport`
- `Message`, `GameApi`

The reorganization is **backward compatible** at the API level.

---

## Drop-in Replacement Design

One of the key goals is enabling **true drop-in replacement** of components. Here's how the architecture achieves this:

### Adding a Web Player Interface

**Use case:** Browser-based game using WebAssembly

**Implementation:**

1. Create `src/player/web.rs`:
   ```rust
   #[cfg(all(feature = "std", target_arch = "wasm32"))]
   use wasm_bindgen::prelude::*;
   use crate::core::{Board, BoardError, GuessResult, BitBoard, BOARD_SIZE, NUM_SHIPS};
   use crate::player::Player;
   use rand::rngs::SmallRng;

   type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

   #[wasm_bindgen]
   pub struct WebPlayer {
       // JavaScript callback for ship placement
       // JavaScript callback for target selection
   }

   impl Player for WebPlayer {
       fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board)
           -> Result<(), BoardError> {
           // Call JavaScript to get ship placements from UI
           // Validate and place ships on board
       }

       fn select_target(
           &mut self,
           rng: &mut SmallRng,
           hits: &BB,
           misses: &BB,
           remaining: &[usize; NUM_SHIPS as usize],
       ) -> (usize, usize) {
           // Call JavaScript to get target from UI
           // Return selected coordinate
       }

       fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult) {
           // Update UI: mark hit/miss on opponent board
       }

       fn handle_opponent_guess(&mut self, coord: (usize, usize), result: GuessResult) {
           // Update UI: show opponent's guess on own board
       }
   }
   ```

2. Register in `src/player/mod.rs`:
   ```rust
   #[cfg(all(feature = "std", target_arch = "wasm32"))]
   pub mod web;

   #[cfg(all(feature = "std", target_arch = "wasm32"))]
   pub use web::WebPlayer;
   ```

3. Use it:
   ```rust
   let player: Box<dyn Player> = Box::new(WebPlayer::new());
   let node = PlayerNode::new(player, GameEngine::new(), transport);
   ```

**Result:** Zero changes to core game logic, PlayerNode, or transport layer. Just one new file.

---

### Adding Bluetooth Transport

**Use case:** Physical battleship boards communicating via BLE

**Implementation:**

1. Create `src/transport/bluetooth.rs`:
   ```rust
   #[cfg(feature = "bluetooth")]
   use btleplug::api::{Central, Peripheral};
   use crate::protocol::Message;
   use crate::transport::Transport;

   pub struct BluetoothTransport {
       peripheral: Peripheral,
       characteristic: Characteristic,
   }

   impl BluetoothTransport {
       pub async fn connect(device_id: &str) -> anyhow::Result<Self> {
           // Scan for BLE devices
           // Connect to specified device
           // Find battleship service/characteristic
       }
   }

   #[async_trait::async_trait]
   impl Transport for BluetoothTransport {
       async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
           let bytes = bincode::serialize(&msg)?;
           self.characteristic.write(&bytes, WriteType::WithResponse).await?;
           Ok(())
       }

       async fn recv(&mut self) -> anyhow::Result<Message> {
           let bytes = self.characteristic.read().await?;
           Ok(bincode::deserialize(&bytes)?)
       }
   }
   ```

2. Register in `src/transport/mod.rs`:
   ```rust
   #[cfg(feature = "bluetooth")]
   pub mod bluetooth;

   #[cfg(feature = "bluetooth")]
   pub use bluetooth::BluetoothTransport;
   ```

3. Add feature to `Cargo.toml`:
   ```toml
   [features]
   bluetooth = ["btleplug"]

   [dependencies]
   btleplug = { version = "0.11", optional = true }
   ```

4. Use it:
   ```rust
   let transport: Box<dyn Transport> =
       Box::new(BluetoothTransport::connect("AA:BB:CC:DD:EE:FF").await?);
   let node = PlayerNode::new(player, GameEngine::new(), transport);
   ```

**Result:** Zero changes to PlayerNode, protocol, or game logic. Just one new file and one feature flag.

---

### Adding Hardware Player (Physical Controls)

**Use case:** Raspberry Pi with LED matrix display and button grid

**Implementation:**

1. Create `src/player/hardware.rs`:
   ```rust
   #[cfg(feature = "hardware")]
   use rppal::gpio::{Gpio, InputPin, OutputPin};
   use crate::core::{Board, BoardError, GuessResult};
   use crate::player::Player;

   pub struct HardwarePlayer {
       gpio: Gpio,
       button_matrix: Vec<Vec<InputPin>>,
       led_matrix: Vec<Vec<OutputPin>>,
   }

   impl Player for HardwarePlayer {
       fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board)
           -> Result<(), BoardError> {
           // Read ship placements from physical board
           // Use rotary encoder + button for orientation
           // Light up LEDs to confirm placement
       }

       fn select_target(
           &mut self,
           rng: &mut SmallRng,
           hits: &BB,
           misses: &BB,
           remaining: &[usize; NUM_SHIPS as usize],
       ) -> (usize, usize) {
           // Scan button matrix for press
           // Show targeting crosshair on LED matrix
           // Return selected coordinate
       }

       fn handle_guess_result(&mut self, coord: (usize, usize), result: GuessResult) {
           // Update LED matrix: red=hit, blue=miss, green=sink
           match result {
               GuessResult::Hit => self.set_led(coord, Color::Red),
               GuessResult::Miss => self.set_led(coord, Color::Blue),
               GuessResult::Sunk(_) => self.set_led(coord, Color::Green),
           }
       }
   }
   ```

2. Register in `src/player/mod.rs`:
   ```rust
   #[cfg(feature = "hardware")]
   pub mod hardware;

   #[cfg(feature = "hardware")]
   pub use hardware::HardwarePlayer;
   ```

**Result:** Physical battleship game with zero changes to core logic.

---

### Usage Pattern: Mix and Match

```rust
// All player types implement same trait
let player: Box<dyn Player> = match config.player_type {
    PlayerType::Cli => Box::new(CliPlayer::new()),
    PlayerType::Ai => Box::new(AiPlayer::new()),
    PlayerType::Web => Box::new(WebPlayer::new()),
    PlayerType::Hardware => Box::new(HardwarePlayer::new()),
};

// All transports implement same trait
let transport: Box<dyn Transport> = match config.connection {
    Connection::Local => Box::new(InMemoryTransport::new()),
    Connection::Tcp(addr) => Box::new(TcpTransport::connect(addr).await?),
    Connection::Bluetooth(dev) => Box::new(BluetoothTransport::connect(dev).await?),
    Connection::WebSocket(url) => Box::new(WebSocketTransport::connect(url).await?),
};

// PlayerNode works with ANY combination:
// - CLI player + TCP transport (human vs human over network)
// - AI player + Bluetooth transport (AI vs physical board)
// - Web player + WebSocket transport (browser-based game)
// - Hardware player + Bluetooth transport (two physical boards)
let node = PlayerNode::new(player, GameEngine::new(), transport);
node.run(&mut rng, first_move).await?;
```

This is the **power of trait-based design**: completely decoupled, composable components.

---

## Benefits Summary

### Improved Organization

✅ **Clear separation**: Core engine vs infrastructure vs application
✅ **Easy navigation**: Related code grouped together (all players in `player/`, all transports in `transport/`)
✅ **Obvious where to add features**: New player? → `player/`. New transport? → `transport/`
✅ **Module boundaries match conceptual boundaries**: Code organization reflects architecture

### Better Modularity

✅ **no_std core isolated**: `core/` module clearly separated, easy to verify no std dependencies
✅ **Player implementations in dedicated files**: Each player type is self-contained
✅ **Transport implementations in dedicated files**: Each transport is self-contained
✅ **Protocol layer clearly separated**: Network-specific code in `protocol/`

### Drop-in Replacement

✅ **Add `player/web.rs`** → implement Player trait → register in mod.rs → done
✅ **Add `transport/bluetooth.rs`** → implement Transport trait → register in mod.rs → done
✅ **No changes to existing code**: Core logic, PlayerNode, protocol remain untouched

### Maintainability

✅ **Dependencies flow in one direction**: Layers depend only downward
✅ **Easy to test each layer independently**: Clear boundaries enable focused testing
✅ **Feature gates naturally align with modules**: `core/` = no gates, `player/cli.rs` = `#[cfg(feature = "std")]`
✅ **Public API stability via root re-exports**: Users continue importing from root

### Extensibility

✅ **Web interface**: Single file in `player/`
✅ **Bluetooth transport**: Single file in `transport/`
✅ **Physical controls**: Single file in `player/`
✅ **WebSocket transport**: Single file in `transport/`
✅ **Future game modes**: Can add without touching core

---

## Migration Risks and Mitigation

### Risk: Breaking Test Imports

**Mitigation:** Root re-exports maintain backward compatibility. Most tests can continue using `use battleship::*;`. A small number may need explicit module paths (`use battleship::core::bitboard::*;`).

**Verification:** Run `cargo test` after each migration step. All 21 tests must pass.

### Risk: Breaking no_std Compatibility

**Mitigation:** Verify `cargo check --no-default-features` succeeds after moving core files. Ensure no std dependencies leak into `core/`.

**Verification:** CI step to build with `--no-default-features`.

### Risk: Import Cycle

**Mitigation:** Strict layer dependency rule prevents cycles:
- Layer 1 (core) depends on nothing
- Layer 2 (player, transport) depends only on Layer 1
- Layer 3 (protocol) depends on Layers 1-2
- Layer 4 (cli) depends on Layers 1-3

**Verification:** Rust compiler will catch any cycles.

### Risk: Lost Work During File Moves

**Mitigation:** Use `git mv` to preserve file history. Commit after each layer is migrated.

**Verification:** Run `git log --follow src/core/board.rs` to confirm history is preserved.

---

## Success Criteria

The reorganization is successful when:

1. ✅ All 21 tests pass
2. ✅ `cargo check --no-default-features` succeeds (no_std core)
3. ✅ `cargo clippy` has zero new warnings
4. ✅ `cargo build --release` produces working binary
5. ✅ Git history preserved for all moved files
6. ✅ Documentation updated with new structure
7. ✅ Example of adding a new player or transport documented

---

## Next Steps

1. **Review this plan** with stakeholders
2. **Implement migration** following the step-by-step checklist (see `migration-checklist.md`)
3. **Verify** all tests pass and functionality is preserved
4. **Document** new structure in README
5. **Demonstrate** adding a new component (e.g., stub WebPlayer)

---

## References

- **Migration Checklist**: See `migration-checklist.md` for detailed implementation steps
- **Before/After Comparison**: See `before-after-comparison.md` for visual structure comparison
- **Original Plan**: See `~/.claude/plans/glistening-dazzling-pinwheel.md`
