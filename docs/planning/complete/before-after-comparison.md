# Before/After Comparison: Repository Structure

This document provides a visual comparison of the repository structure before and after the reorganization.

---

## Before: Flat Structure

### Directory Tree

```
battleship/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
│
├── src/
│   ├── lib.rs                  (112 lines)  - Root library
│   ├── main.rs                 (407 lines)  - CLI binary
│   │
│   ├── ai.rs                   (184 lines)  - AI probability calculations
│   ├── bitboard.rs             (366 lines)  - Generic bitboard
│   ├── board.rs                (217 lines)  - Board state
│   ├── cli.rs                  (32 lines)   - CLI command definitions
│   ├── common.rs               (66 lines)   - GuessResult, errors
│   ├── config.rs               (25 lines)   - Game constants
│   ├── domain.rs               (59 lines)   - Serializable types
│   ├── game.rs                 (200 lines)  - GameEngine
│   ├── interface_cli.rs        (36 lines)   - Display helpers
│   ├── player.rs               (31 lines)   - Player trait
│   ├── player_ai.rs            (45 lines)   - AI player
│   ├── player_cli.rs           (375 lines)  - CLI player
│   ├── player_node.rs          (272 lines)  - Player orchestrator
│   ├── protocol.rs             (76 lines)   - Message enum
│   ├── ship.rs                 (200 lines)  - Ship placement
│   ├── skeleton.rs             (141 lines)  - Server RPC
│   ├── stub.rs                 (105 lines)  - Client RPC
│   │
│   └── transport/
│       ├── mod.rs              (16 lines)   - Transport trait
│       ├── in_memory.rs        (153 lines)  - In-memory transport
│       ├── tcp.rs              (241 lines)  - TCP transport
│       └── heartbeat.rs        (178 lines)  - Heartbeat wrapper
│
└── tests/
    ├── ai_game_tests.rs
    ├── bitboard_proptest.rs
    ├── bitboard_tests.rs
    ├── board_proptest.rs
    ├── board_tests.rs
    ├── game_tests.rs
    ├── heartbeat_integration_tests.rs
    ├── heartbeat_transport_tests.rs
    ├── in_memory_ai_game_test.rs
    ├── in_memory_transport_test.rs
    ├── player_node_robustness_tests.rs
    ├── protocol_hardening_tests.rs
    ├── ship_placement_tests.rs
    ├── ship_tests.rs
    ├── tcp_integration_tests.rs
    ├── tcp_server_tests.rs
    ├── tcp_timeout_tests.rs
    └── tcp_transport_tests.rs
    └── ... (21+ total)
```

### File Count

- **Root src/ directory:** 19 Rust files (flat)
- **transport/ subdirectory:** 4 files
- **Total:** 23 files

### Pain Points

❌ **Hard to navigate**: 19 files at the same level
❌ **Unclear boundaries**: Which files are core logic vs infrastructure?
❌ **no_std unclear**: Which files work without std?
❌ **Difficult to extend**: Where should `web.rs` or `bluetooth.rs` go?
❌ **Mixed concerns**: Game logic, networking, CLI all mixed together

---

## After: Layered Architecture

### Directory Tree

```
battleship/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
│
├── docs/
│   └── planning/               🆕 Planning documents (gitignored)
│       ├── reorganization-plan.md
│       ├── migration-checklist.md
│       └── before-after-comparison.md
│
├── src/
│   ├── lib.rs                  (Updated)  - Layer-based public API
│   ├── main.rs                 (Minimal)  - Just calls cli::run()
│   │
│   ├── core/                   🆕 Layer 1: Game engine (no_std)
│   │   ├── mod.rs              - Module orchestration
│   │   ├── ai.rs               (184 lines)  - AI probability calculations
│   │   ├── bitboard.rs         (366 lines)  - Generic bitboard
│   │   ├── board.rs            (217 lines)  - Board state
│   │   ├── common.rs           (66 lines)   - GuessResult, errors
│   │   ├── config.rs           (25 lines)   - Game constants
│   │   ├── game.rs             (200 lines)  - GameEngine
│   │   └── ship.rs             (200 lines)  - Ship placement
│   │
│   ├── player/                 🆕 Layer 2a: Player abstractions
│   │   ├── mod.rs              - Player trait definition
│   │   ├── ai.rs               (45 lines)   - AI player (was player_ai.rs)
│   │   ├── cli.rs              (375 lines)  - CLI player (was player_cli.rs)
│   │   └── node.rs             (272 lines)  - Orchestrator (was player_node.rs)
│   │
│   ├── transport/              ✓ Layer 2b: Transport abstractions (already exists)
│   │   ├── mod.rs              (16 lines)   - Transport trait
│   │   ├── in_memory.rs        (153 lines)  - In-memory transport
│   │   ├── tcp.rs              (241 lines)  - TCP transport
│   │   └── heartbeat.rs        (178 lines)  - Heartbeat wrapper
│   │
│   ├── protocol/               🆕 Layer 3: Network protocol & RPC
│   │   ├── mod.rs              (76 lines)   - Message enum, GameApi (was protocol.rs)
│   │   ├── domain.rs           (59 lines)   - Serializable types
│   │   ├── skeleton.rs         (141 lines)  - Server RPC
│   │   └── stub.rs             (105 lines)  - Client RPC
│   │
│   └── cli/                    🆕 Layer 4: Application interface
│       ├── mod.rs              - CLI struct, command definitions (from cli.rs)
│       ├── commands.rs         (~350 lines) - Command handlers (from main.rs)
│       └── interface.rs        (36 lines)   - Display helpers (was interface_cli.rs)
│
└── tests/
    ├── ai_game_tests.rs        (Updated imports)
    ├── bitboard_proptest.rs    (Updated imports)
    ├── bitboard_tests.rs       (Updated imports)
    └── ... (21+ total, all updated)
```

### Module Count

- **Layer 1 (core):** 8 files (7 + mod.rs)
- **Layer 2a (player):** 4 files (3 + mod.rs)
- **Layer 2b (transport):** 4 files (3 + mod.rs)
- **Layer 3 (protocol):** 4 files (3 + mod.rs)
- **Layer 4 (cli):** 3 files (2 + mod.rs)
- **Total:** 23 files (same number, better organized)

### Benefits

✅ **Clear navigation**: 5 top-level modules, each with clear purpose
✅ **Obvious boundaries**: core/ = no_std, everything else = std
✅ **Easy to extend**: New player? → player/. New transport? → transport/
✅ **Separated concerns**: Game logic in core/, networking in protocol/, app in cli/
✅ **Maintainable**: Dependencies flow in one direction (downward)

---

## Side-by-Side Comparison

### Core Game Logic

| Before | After |
|--------|-------|
| `src/ai.rs` | `src/core/ai.rs` |
| `src/bitboard.rs` | `src/core/bitboard.rs` |
| `src/board.rs` | `src/core/board.rs` |
| `src/common.rs` | `src/core/common.rs` |
| `src/config.rs` | `src/core/config.rs` |
| `src/game.rs` | `src/core/game.rs` |
| `src/ship.rs` | `src/core/ship.rs` |

**Improvement:** All core logic in one module, clearly marked as no_std compatible.

---

### Player Implementations

| Before | After |
|--------|-------|
| `src/player.rs` | `src/player/mod.rs` (trait definition) |
| `src/player_ai.rs` | `src/player/ai.rs` |
| `src/player_cli.rs` | `src/player/cli.rs` |
| `src/player_node.rs` | `src/player/node.rs` |

**Improvement:** All player-related code in one module. Easy to find and extend.

---

### Transport Implementations

| Before | After |
|--------|-------|
| `src/transport/mod.rs` | `src/transport/mod.rs` ✓ (unchanged) |
| `src/transport/in_memory.rs` | `src/transport/in_memory.rs` ✓ |
| `src/transport/tcp.rs` | `src/transport/tcp.rs` ✓ |
| `src/transport/heartbeat.rs` | `src/transport/heartbeat.rs` ✓ |

**Improvement:** Already well-organized, now consistent with other layers.

---

### Protocol Layer

| Before | After |
|--------|-------|
| `src/protocol.rs` | `src/protocol/mod.rs` |
| `src/domain.rs` | `src/protocol/domain.rs` |
| `src/skeleton.rs` | `src/protocol/skeleton.rs` |
| `src/stub.rs` | `src/protocol/stub.rs` |

**Improvement:** All protocol-related code grouped together.

---

### CLI Application

| Before | After |
|--------|-------|
| `src/main.rs` (407 lines) | `src/main.rs` (~20 lines, minimal) |
| `src/cli.rs` | `src/cli/mod.rs` |
| (embedded in main.rs) | `src/cli/commands.rs` |
| `src/interface_cli.rs` | `src/cli/interface.rs` |

**Improvement:** CLI code extracted from main.rs, organized as a module.

---

## Import Changes

### Before (Using Flat Structure)

```rust
// In a test file or application code:
use battleship::board::*;
use battleship::ship::*;
use battleship::game::*;
use battleship::ai::*;
use battleship::player_ai::AiPlayer;
use battleship::player_cli::CliPlayer;
use battleship::transport::tcp::TcpTransport;
use battleship::protocol::*;
```

### After (Using Layered Structure)

**Option 1: Root re-exports (backward compatible):**
```rust
use battleship::*;  // Still works! Imports core types + Player trait
```

**Option 2: Explicit module paths:**
```rust
use battleship::core::{Board, Ship, GameEngine, ai::*};
use battleship::player::{AiPlayer, CliPlayer};
use battleship::transport::tcp::TcpTransport;
use battleship::protocol::*;
```

**Recommendation:** Use Option 1 for simple cases, Option 2 for clarity.

---

## Dependency Graph

### Before: Unclear Dependencies

```
┌─────────────────────────────────────────────────┐
│ main.rs (407 lines)                             │
│ - Command parsing                               │
│ - Game setup                                    │
│ - Network handling                              │
│ - All mixed together                            │
└─────────────────────────────────────────────────┘
         ↓ (uses everything)
┌─────────────────────────────────────────────────┐
│ Flat src/ directory (19 files)                  │
│ - Dependencies unclear                          │
│ - Circular references possible                  │
│ - Hard to understand what depends on what       │
└─────────────────────────────────────────────────┘
```

### After: Clear Layered Dependencies

```
┌─────────────────────────────────────────────────┐
│ Layer 4: cli/ (Application)                     │
│ - User interface                                │
│ - Command handling                              │
└─────────────────────────────────────────────────┘
         ↓ depends on
┌─────────────────────────────────────────────────┐
│ Layer 3: protocol/ (Network Protocol)           │
│ - Message definitions                           │
│ - RPC framework                                 │
└─────────────────────────────────────────────────┘
         ↓ depends on
┌─────────────────────────────────────────────────┐
│ Layer 2: player/ + transport/ (Abstractions)    │
│ - Player trait + implementations                │
│ - Transport trait + implementations             │
└─────────────────────────────────────────────────┘
         ↓ depends on
┌─────────────────────────────────────────────────┐
│ Layer 1: core/ (Game Engine)                    │
│ - Pure game logic (no_std)                      │
│ - No dependencies on upper layers               │
└─────────────────────────────────────────────────┘
```

**Key principle:** Dependencies only flow downward, never upward or sideways.

---

## Feature Gate Organization

### Before: Scattered Feature Gates

```rust
// src/player_ai.rs
use rand::prelude::*;  // Works in no_std with alloc

// src/player_cli.rs
#[cfg(feature = "std")]  // Requires std
use std::io::{self, Write};

// src/transport/tcp.rs
#[cfg(feature = "std")]  // Requires std
use tokio::net::TcpStream;

// Hard to see which files are no_std compatible
```

### After: Clear Module-Level Feature Gates

```rust
// src/core/mod.rs
// No #[cfg] needed - entire module is no_std compatible

// src/player/mod.rs
pub mod ai;  // no_std compatible

#[cfg(feature = "std")]
pub mod cli;  // std required

#[cfg(feature = "std")]
pub mod node;  // std required

// src/transport/mod.rs
#![cfg(feature = "std")]  // Entire module requires std

// src/protocol/mod.rs
#![cfg(feature = "std")]  // Entire module requires std

// src/cli/mod.rs
#![cfg(feature = "std")]  // Entire module requires std
```

**Benefit:** Immediately obvious which modules work in no_std environments.

---

## Extension Scenarios

### Scenario 1: Adding a Web Player

**Before (Flat Structure):**
- Create `src/player_web.rs` (clutters root)
- Update `src/lib.rs` to declare `pub mod player_web;`
- Update `src/main.rs` or relevant files to use it
- Unclear where web-specific code should go

**After (Layered Structure):**
1. Create `src/player/web.rs`
2. Add `pub mod web;` to `src/player/mod.rs`
3. Done! Automatically available via `use battleship::player::WebPlayer;`

**Lines changed:** 2 (create file + add one line to mod.rs)

---

### Scenario 2: Adding Bluetooth Transport

**Before (Flat Structure):**
- Create `src/transport/bluetooth.rs` (already in subfolder, good!)
- Update `src/transport/mod.rs`
- Already well-organized for this

**After (Layered Structure):**
- Same as before (transport was already modular)
- Now consistent with player organization

**Lines changed:** 2 (create file + add one line to mod.rs)

---

### Scenario 3: Adding a Remote API Server

**Before (Flat Structure):**
- Unclear where to put it
- `src/api_server.rs`? `src/server.rs`? `src/remote.rs`?
- Might conflict with existing protocol/skeleton/stub

**After (Layered Structure):**
- Clear location: `src/protocol/api_server.rs` (extends protocol layer)
- Or `src/cli/server.rs` (if it's an application-level command)

**Benefit:** Obvious where new code belongs.

---

## Testing Organization

### Before: Tests Import from Flat Structure

```rust
// tests/board_tests.rs
use battleship::board::*;
use battleship::ship::*;
use battleship::bitboard::*;
use battleship::common::*;

// Tests directly reference top-level modules
```

### After: Tests Import from Layered Structure

```rust
// tests/board_tests.rs
// Option 1: Use root re-exports
use battleship::*;

// Option 2: Explicit module paths
use battleship::core::{Board, Ship, BitBoard, GuessResult};

// Tests reference logical groupings
```

**Benefit:** Tests can import entire layers (e.g., `use battleship::core::*;`) for comprehensive testing.

---

## Code Navigation Improvements

### Before: Finding Player Code

1. Open `src/`
2. Scan 19 files for `player*`
3. Find `player.rs`, `player_ai.rs`, `player_cli.rs`, `player_node.rs`
4. Open each individually

**Steps:** 4

### After: Finding Player Code

1. Open `src/player/`
2. See all player-related files immediately

**Steps:** 1

**Time saved:** 50%+

---

### Before: Understanding no_std Compatibility

1. Open each file individually
2. Check for `#[cfg(feature = "std")]` or std imports
3. Manually track which files are no_std compatible
4. Easy to miss dependencies

**Effort:** High, error-prone

### After: Understanding no_std Compatibility

1. Look at module structure
2. `core/` = no_std
3. Everything else = std required (or feature-gated)

**Effort:** Instant, obvious from directory structure

---

## Summary of Changes

| Aspect | Before | After | Improvement |
|--------|--------|-------|-------------|
| **File count** | 23 files | 23 files | Same |
| **Top-level files** | 19 in src/ | 5 modules | 74% reduction |
| **no_std visibility** | Scattered `#[cfg]` | Entire `core/` module | Obvious |
| **Extension** | Unclear location | Clear module (player/ or transport/) | Guided |
| **Navigation** | Scan 19 files | Browse 5 modules | Faster |
| **Dependency flow** | Unclear | One-directional (downward) | Maintainable |
| **Public API** | Flat imports | Same (backward compatible) | No change |
| **Tests** | 21 files | 21 files (updated imports) | Same coverage |
| **Lines of code** | ~3,500 | ~3,500 | Same |

---

## Visual: Code Distribution

### Before (Flat)

```
src/
├── [19 files]  ← Everything at same level
└── transport/
    └── [4 files]
```

### After (Layered)

```
src/
├── core/       [8 files]  ← Layer 1: Pure logic
├── player/     [4 files]  ← Layer 2a: Player abstractions
├── transport/  [4 files]  ← Layer 2b: Transport abstractions
├── protocol/   [4 files]  ← Layer 3: Network protocol
└── cli/        [3 files]  ← Layer 4: Application
```

---

## Architectural Clarity

### Before: Implicit Architecture

The architecture existed in developers' heads and scattered `#[cfg]` attributes:

```
Core logic (no_std) ← Not obvious from file structure
   ↓
Player abstractions ← player*.rs files
   ↓
Transport abstractions ← transport/ folder (good!)
   ↓
Protocol layer ← protocol.rs, domain.rs, skeleton.rs, stub.rs scattered
   ↓
CLI application ← main.rs + cli.rs + interface_cli.rs
```

### After: Explicit Architecture

The architecture is **visible in the directory structure**:

```
core/       ← Layer 1: Core (no_std) - directory name makes it obvious
   ↓
player/     ← Layer 2a: Players - grouped together
transport/  ← Layer 2b: Transports - grouped together
   ↓
protocol/   ← Layer 3: Protocol - all networking code together
   ↓
cli/        ← Layer 4: CLI - application layer
```

**Benefit:** New contributors immediately understand the architecture.

---

## Onboarding Experience

### Before: New Contributor

"I want to add a web-based player. Where should I put it?"

1. Open `src/`
2. See 19 files
3. Not sure: `player_web.rs`? `web.rs`? `web_player.rs`?
4. Check existing patterns (`player_ai.rs`, `player_cli.rs`)
5. Create `src/player_web.rs`
6. Update `lib.rs`
7. Hope it works

**Time:** 15-30 minutes of exploration

### After: New Contributor

"I want to add a web-based player. Where should I put it?"

1. Open `src/`
2. See `player/` module
3. Open `player/`
4. See `ai.rs`, `cli.rs`, `node.rs`, `mod.rs`
5. Pattern is obvious: Create `web.rs`
6. Add one line to `mod.rs`
7. Done

**Time:** 5 minutes

**Improvement:** 3-6x faster onboarding

---

## Conclusion

The reorganization transforms the repository from a **flat collection of files** into a **layered architecture** with clear boundaries:

✅ **Same number of files** (23), better organized
✅ **Same lines of code** (~3,500), better grouped
✅ **Same functionality**, clearer structure
✅ **Backward compatible** public API
✅ **Easier to navigate** (5 modules vs 19 files)
✅ **Obvious no_std boundary** (core/ module)
✅ **Clear extension points** (player/, transport/)
✅ **Maintainable dependencies** (one-directional flow)

**Result:** A more professional, maintainable, and extensible codebase that scales as the project grows.
