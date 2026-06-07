# Migration Checklist: Battleship Repository Reorganization

**Status:** Not Started
**Estimated Time:** 3-4 hours
**Risk Level:** Medium (file moves, import updates)

---

## Pre-Migration Setup

### ☐ Step 0.1: Verify Clean Working Directory

```bash
git status
```

**Expected:** No uncommitted changes

**If there are changes:**
- Commit them: `git add . && git commit -m "Pre-reorganization snapshot"`
- Or stash them: `git stash`

---

### ☐ Step 0.2: Create Feature Branch

```bash
git checkout -b refactor/layered-architecture
```

---

### ☐ Step 0.3: Verify All Tests Pass (Baseline)

```bash
cargo test
```

**Expected:** All 21 tests pass

**If tests fail:** Fix issues before proceeding. The baseline must be green.

---

### ☐ Step 0.4: Verify no_std Build (Baseline)

```bash
cargo check --no-default-features
```

**Expected:** Success

---

### ☐ Step 0.5: Verify Clippy (Baseline)

```bash
cargo clippy -- -D warnings
```

**Expected:** Zero warnings

---

## Phase 1: Create Core Module

### ☐ Step 1.1: Create Core Module Directory

```bash
mkdir src/core
```

---

### ☐ Step 1.2: Create Core Module Definition

Create `src/core/mod.rs`:

```rust
//! Core battleship game engine (no_std compatible)
//!
//! This module contains the pure game logic with zero external dependencies
//! (except num-traits, libm, rand::alloc). It can be used in embedded systems
//! or compiled to WebAssembly.

pub mod ai;
pub mod bitboard;
pub mod board;
pub mod common;
pub mod config;
pub mod game;
pub mod ship;

// Re-export commonly used types
pub use ai::{calc_pdf, choose_best_square};
pub use bitboard::{BitBoard, BitBoardError};
pub use board::{Board, BoardState};
pub use common::{BoardError, GuessResult};
pub use config::*;
pub use game::{GameEngine, GameState, GameStatus, GuessBoardState};
pub use ship::{Orientation, Ship, ShipDef, ShipState};
```

**Verification:**
```bash
# File should exist
ls src/core/mod.rs
```

---

### ☐ Step 1.3: Move Core Files

Use `git mv` to preserve history:

```bash
git mv src/bitboard.rs src/core/bitboard.rs
git mv src/board.rs src/core/board.rs
git mv src/ship.rs src/core/ship.rs
git mv src/game.rs src/core/game.rs
git mv src/ai.rs src/core/ai.rs
git mv src/config.rs src/core/config.rs
git mv src/common.rs src/core/common.rs
```

**Verification:**
```bash
ls src/core/*.rs | wc -l  # Should be 8 (including mod.rs)
```

---

### ☐ Step 1.4: Update Imports Within Core Files

For each file in `src/core/`, update imports:

**Pattern to find:**
- `use crate::bitboard::*` → `use crate::core::bitboard::*` or `use super::bitboard::*`
- `use crate::board::*` → `use super::board::*`
- `use crate::ship::*` → `use super::ship::*`
- `use crate::common::*` → `use super::common::*`
- `use crate::config::*` → `use super::config::*`

**Files to update:**
1. `src/core/bitboard.rs` - Check imports (likely has none)
2. `src/core/board.rs` - Update imports of bitboard, ship, common, config
3. `src/core/ship.rs` - Update imports of bitboard, config, common
4. `src/core/game.rs` - Update imports of board, ship, bitboard, common, config
5. `src/core/ai.rs` - Update imports of bitboard, config
6. `src/core/config.rs` - Check imports (likely has none)
7. `src/core/common.rs` - Check imports (likely has none)

**Recommended approach:**
- Use relative imports within core: `use super::bitboard::*`
- This makes it clear that imports are within the same module

---

### ☐ Step 1.5: Update lib.rs to Include Core Module

Edit `src/lib.rs`:

**Add near the top (after feature gates):**
```rust
// ========================================
// Layer 1: Core game engine (no_std)
// ========================================
pub mod core;

// Convenience re-exports of core types
pub use core::{
    ai::*, bitboard::*, board::*, common::*, config::*, game::*, ship::*,
};
```

**Remove old module declarations:**
```rust
// DELETE THESE LINES:
// pub mod ai;
// pub mod bitboard;
// pub mod board;
// pub mod common;
// pub mod config;
// pub mod game;
// pub mod ship;
```

---

### ☐ Step 1.6: Verify Core Module Compiles

```bash
cargo check --no-default-features
```

**Expected:** Success (core is no_std compatible)

**If errors:** Fix import issues in core files

---

### ☐ Step 1.7: Commit Core Module

```bash
git add src/core/
git add src/lib.rs
git commit -m "Refactor: Extract core game engine into core/ module

- Move bitboard, board, ship, game, ai, config, common to src/core/
- Update imports to use relative paths within core
- Re-export core types from lib.rs for backward compatibility
- Verify no_std compatibility maintained"
```

---

## Phase 2: Create Player Module

### ☐ Step 2.1: Create Player Module Directory

```bash
mkdir src/player
```

---

### ☐ Step 2.2: Extract Player Trait into Module

Read `src/player.rs` and extract the trait definition into `src/player/mod.rs`:

```rust
//! Player trait and implementations
//!
//! This module defines the Player trait and provides concrete implementations:
//! - AiPlayer: Probability-based AI using hunt/target mode
//! - CliPlayer: Interactive command-line player
//! - PlayerNode: Orchestrator combining Player + GameEngine + Transport

use crate::core::{BitBoard, Board, BoardError, GuessResult, BOARD_SIZE, NUM_SHIPS};
use rand::rngs::SmallRng;

type BB = BitBoard<u128, { BOARD_SIZE as usize }>;

/// Interface implemented by different player types.
///
/// A Player is responsible for:
/// - Placing ships on the board
/// - Selecting targets to attack
/// - Handling feedback from guesses
pub trait Player: Send {
    /// Place ships on the board. The player can use the RNG for random placement.
    fn place_ships(&mut self, rng: &mut SmallRng, board: &mut Board) -> Result<(), BoardError>;

    /// Select the next target coordinate to attack.
    fn select_target(
        &mut self,
        rng: &mut SmallRng,
        hits: &BB,
        misses: &BB,
        remaining: &[usize; NUM_SHIPS as usize],
    ) -> (usize, usize);

    /// Handle the result of this player's guess.
    fn handle_guess_result(&mut self, _coord: (usize, usize), _result: GuessResult) {}

    /// Handle feedback about opponent's guess on this player's board.
    fn handle_opponent_guess(&mut self, _coord: (usize, usize), _result: GuessResult) {}
}

// Re-export implementations
pub mod ai;
pub use ai::AiPlayer;

#[cfg(feature = "std")]
pub mod cli;
#[cfg(feature = "std")]
pub use cli::CliPlayer;

#[cfg(feature = "std")]
pub mod node;
#[cfg(feature = "std")]
pub use node::PlayerNode;
```

---

### ☐ Step 2.3: Move Player Implementation Files

```bash
git mv src/player_ai.rs src/player/ai.rs
git mv src/player_cli.rs src/player/cli.rs
git mv src/player_node.rs src/player/node.rs
```

**Verification:**
```bash
ls src/player/*.rs | wc -l  # Should be 4 (mod.rs + ai.rs + cli.rs + node.rs)
```

---

### ☐ Step 2.4: Update Imports in Player Files

**File: `src/player/ai.rs`**

Update imports:
```rust
// OLD:
use crate::player::Player;
use crate::ai::*;
use crate::board::*;
use crate::bitboard::*;
use crate::config::*;
use crate::common::*;

// NEW:
use super::Player;  // Player trait is in same module
use crate::core::{
    ai::*, board::*, bitboard::*, config::*, common::*, Board, BitBoard, BoardError, GuessResult,
};
```

**File: `src/player/cli.rs`**

Update imports:
```rust
// OLD:
use crate::player::Player;
use crate::board::*;
use crate::ship::*;
use crate::bitboard::*;
use crate::common::*;
use crate::config::*;
use crate::interface_cli::*;  // Will update later when cli module is created

// NEW:
use super::Player;
use crate::core::{
    board::*, ship::*, bitboard::*, common::*, config::*,
    Board, Ship, BitBoard, GuessResult, BoardError, BOARD_SIZE, NUM_SHIPS,
};
// Leave interface_cli import as-is for now, will fix in Phase 5
```

**File: `src/player/node.rs`**

Update imports:
```rust
// OLD:
use crate::player::Player;
use crate::game::*;
use crate::transport::Transport;
use crate::protocol::*;
use crate::common::*;
use crate::config::*;

// NEW:
use super::Player;
use crate::core::{
    game::*, common::*, config::*, GameEngine, GuessResult, BoardError,
};
use crate::transport::Transport;
use crate::protocol::*;
```

---

### ☐ Step 2.5: Remove Old player.rs

```bash
git rm src/player.rs
```

---

### ☐ Step 2.6: Update lib.rs to Include Player Module

Edit `src/lib.rs`:

**Add:**
```rust
// ========================================
// Layer 2: Player & Transport
// ========================================
pub mod player;
pub use player::{AiPlayer, Player};

#[cfg(feature = "std")]
pub use player::{CliPlayer, PlayerNode};
```

**Remove old declarations:**
```rust
// DELETE THESE LINES:
// pub mod player;
// pub mod player_ai;
// pub mod player_cli;
// pub mod player_node;
```

---

### ☐ Step 2.7: Verify Player Module Compiles

```bash
cargo check
```

**Expected:** May have errors related to `interface_cli` - that's expected and will be fixed in Phase 5

---

### ☐ Step 2.8: Commit Player Module

```bash
git add src/player/
git add src/lib.rs
git commit -m "Refactor: Extract player implementations into player/ module

- Move player trait to player/mod.rs
- Move player_ai.rs → player/ai.rs
- Move player_cli.rs → player/cli.rs
- Move player_node.rs → player/node.rs
- Update imports to use core:: and super::
- Re-export player types from lib.rs"
```

---

## Phase 3: Organize Protocol Module

### ☐ Step 3.1: Create Protocol Module Directory

```bash
mkdir src/protocol
```

---

### ☐ Step 3.2: Move Protocol Files

```bash
git mv src/domain.rs src/protocol/domain.rs
git mv src/skeleton.rs src/protocol/skeleton.rs
git mv src/stub.rs src/protocol/stub.rs
```

---

### ☐ Step 3.3: Create Protocol Module Definition

Read `src/protocol.rs` and move its content into `src/protocol/mod.rs`:

```rust
//! Network protocol definitions and RPC framework
//!
//! This module defines the protocol for communicating between game instances:
//! - Message enum: All protocol messages (Handshake, Guess, StatusResp, etc.)
//! - GameApi trait: RPC interface for game operations
//! - Skeleton: Server-side RPC handler
//! - Stub: Client-side RPC proxy
//! - Domain types: Serializable versions of game types

#![cfg(feature = "std")]

pub mod domain;
pub mod skeleton;
pub mod stub;

pub use async_trait;
use serde::{Deserialize, Serialize};

use domain::*;

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u8 = 1;

/// Messages exchanged between game engine and remote client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Initial handshake with protocol version
    Handshake { version: u8 },

    /// Version-tagged messages
    #[serde(untagged)]
    Versioned(VersionedMessage),

    /// Heartbeat for connection monitoring (not version-tagged)
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedMessage {
    pub version: u8,
    pub seq: u32,
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    // ... (copy from original protocol.rs)
}

/// RPC interface for game operations
#[async_trait::async_trait]
pub trait GameApi: Send + Sync {
    // ... (copy from original protocol.rs)
}

// Re-exports
pub use skeleton::Skeleton;
pub use stub::Stub;
```

(Copy the full content from `src/protocol.rs`)

---

### ☐ Step 3.4: Remove Old protocol.rs

```bash
git rm src/protocol.rs
```

---

### ☐ Step 3.5: Update Imports in Protocol Files

**File: `src/protocol/domain.rs`**

```rust
// OLD:
use crate::ship::*;
use crate::board::*;
use crate::common::*;

// NEW:
use crate::core::{ship::*, board::*, common::*, Ship, Board, GuessResult};
```

**File: `src/protocol/skeleton.rs`**

```rust
// OLD:
use crate::protocol::*;
use crate::domain::*;
use crate::transport::Transport;
use crate::game::*;
use crate::common::*;

// NEW:
use super::{Message, GameApi, VersionedMessage, MessagePayload, PROTOCOL_VERSION};
use super::domain::*;
use crate::transport::Transport;
use crate::core::{game::*, common::*, GameEngine, GuessResult, BoardError};
```

**File: `src/protocol/stub.rs`**

```rust
// OLD:
use crate::protocol::*;
use crate::domain::*;
use crate::transport::Transport;
use crate::ship::Ship;
use crate::common::GuessResult;

// NEW:
use super::{Message, GameApi, VersionedMessage, MessagePayload, PROTOCOL_VERSION};
use super::domain::*;
use crate::transport::Transport;
use crate::core::{Ship, GuessResult};
```

---

### ☐ Step 3.6: Update lib.rs to Include Protocol Module

Edit `src/lib.rs`:

**Add:**
```rust
// ========================================
// Layer 3: Protocol & RPC
// ========================================
#[cfg(feature = "std")]
pub mod protocol;

#[cfg(feature = "std")]
pub use protocol::{domain, GameApi, Message, Skeleton, Stub, PROTOCOL_VERSION};
```

**Remove old declarations:**
```rust
// DELETE THESE LINES:
// pub mod protocol;
// pub mod domain;
// pub mod skeleton;
// pub mod stub;
```

---

### ☐ Step 3.7: Verify Protocol Module Compiles

```bash
cargo check
```

**Expected:** Success (or minor import issues to fix)

---

### ☐ Step 3.8: Commit Protocol Module

```bash
git add src/protocol/
git add src/lib.rs
git commit -m "Refactor: Organize protocol layer into protocol/ module

- Move protocol.rs content → protocol/mod.rs
- Move domain.rs, skeleton.rs, stub.rs → protocol/
- Update imports to use core:: and super::
- Re-export protocol types from lib.rs"
```

---

## Phase 4: Verify Transport Module

The transport module already exists in `src/transport/`, so we just need to verify it's properly integrated.

### ☐ Step 4.1: Verify Transport Module Structure

```bash
ls src/transport/
```

**Expected files:**
- `mod.rs`
- `in_memory.rs`
- `tcp.rs`
- `heartbeat.rs`

---

### ☐ Step 4.2: Update Imports in Transport Files (if needed)

Check each transport file for references to old paths and update if needed:

**Pattern:**
- `use crate::protocol::*` → `use crate::protocol::Message;`
- `use crate::common::*` → `use crate::core::common::*;`

---

### ☐ Step 4.3: Verify lib.rs Transport Export

Edit `src/lib.rs` to ensure transport is properly exported:

```rust
#[cfg(feature = "std")]
pub mod transport;

#[cfg(feature = "std")]
pub use transport::{heartbeat::HeartbeatTransport, tcp::TcpTransport, Transport};
```

**Note:** InMemoryTransport is typically not re-exported (internal use only)

---

### ☐ Step 4.4: Commit Transport Updates (if any)

```bash
git add src/transport/
git add src/lib.rs
git commit -m "Refactor: Update transport module imports for new structure"
```

---

## Phase 5: Create CLI Module

### ☐ Step 5.1: Create CLI Module Directory

```bash
mkdir src/cli
```

---

### ☐ Step 5.2: Move Interface File

```bash
git mv src/interface_cli.rs src/cli/interface.rs
```

---

### ☐ Step 5.3: Extract Command Definitions from cli.rs

Read `src/cli.rs` and move content into `src/cli/mod.rs`:

```rust
//! Command-line interface

#![cfg(feature = "std")]

pub mod commands;
pub mod interface;

pub use interface::*;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum PlayerType {
    Human,
    Ai,
}

#[derive(Parser)]
pub enum Commands {
    // ... (copy from original cli.rs)
}

/// Main CLI entry point
pub async fn run() -> anyhow::Result<()> {
    commands::execute().await
}
```

---

### ☐ Step 5.4: Extract Command Handlers from main.rs

Create `src/cli/commands.rs`:

Extract the command handling logic from `src/main.rs` (the match statements and helper functions) into this file:

```rust
//! CLI command handlers

use super::{Cli, Commands, PlayerType};
use crate::core::*;
use crate::player::*;
use crate::transport::*;
use crate::protocol::*;
use anyhow::Result;
use clap::Parser;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub async fn execute() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Local { player1, player2 } => {
            handle_local(player1, player2).await
        }
        Commands::TcpServer { port, player } => {
            handle_tcp_server(port, player).await
        }
        Commands::TcpClient { addr, player } => {
            handle_tcp_client(addr, player).await
        }
    }
}

async fn handle_local(player1: PlayerType, player2: PlayerType) -> Result<()> {
    // ... (extract from main.rs)
}

async fn handle_tcp_server(port: u16, player: PlayerType) -> Result<()> {
    // ... (extract from main.rs)
}

async fn handle_tcp_client(addr: SocketAddr, player: PlayerType) -> Result<()> {
    // ... (extract from main.rs)
}

// Helper functions
fn create_player(player_type: PlayerType) -> Box<dyn Player> {
    // ... (extract from main.rs)
}
```

---

### ☐ Step 5.5: Simplify main.rs

Edit `src/main.rs` to just call the CLI:

```rust
#[cfg(not(feature = "std"))]
fn main() {
    // no_std builds have no CLI
}

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    battleship::cli::run().await
}
```

---

### ☐ Step 5.6: Remove Old cli.rs

```bash
git rm src/cli.rs
```

---

### ☐ Step 5.7: Update lib.rs to Include CLI Module

Edit `src/lib.rs`:

**Add:**
```rust
// ========================================
// Layer 4: CLI (not exported by default, used by binary)
// ========================================
#[cfg(feature = "std")]
pub mod cli;
```

**Remove old declarations:**
```rust
// DELETE:
// pub mod cli;
// pub mod interface_cli;
```

---

### ☐ Step 5.8: Update player/cli.rs Import

Edit `src/player/cli.rs`:

```rust
// OLD:
use crate::interface_cli::*;

// NEW:
use crate::cli::interface::*;
```

---

### ☐ Step 5.9: Verify CLI Module Compiles

```bash
cargo check
cargo build --release
```

**Expected:** Success

---

### ☐ Step 5.10: Commit CLI Module

```bash
git add src/cli/
git add src/lib.rs
git add src/main.rs
git add src/player/cli.rs
git commit -m "Refactor: Extract CLI into cli/ module

- Move interface_cli.rs → cli/interface.rs
- Extract command definitions from cli.rs → cli/mod.rs
- Extract command handlers from main.rs → cli/commands.rs
- Simplify main.rs to just call cli::run()
- Update player/cli.rs to use cli::interface"
```

---

## Phase 6: Update Test Imports

### ☐ Step 6.1: Identify Test Files Needing Updates

```bash
ls tests/*.rs
```

Expected ~21 test files.

---

### ☐ Step 6.2: Update Test Imports

For each test file, update imports to use new module paths:

**Pattern:**
```rust
// OLD:
use battleship::bitboard::*;
use battleship::board::*;
use battleship::game::*;
use battleship::ship::*;
use battleship::ai::*;
use battleship::player_ai::AiPlayer;
use battleship::transport::tcp::*;

// NEW (Option 1: Use root re-exports):
use battleship::*;

// NEW (Option 2: Explicit module paths):
use battleship::core::bitboard::*;
use battleship::core::board::*;
use battleship::core::game::*;
use battleship::core::ship::*;
use battleship::core::ai::*;
use battleship::player::AiPlayer;
use battleship::transport::tcp::*;
```

**Recommended:** Try Option 1 first (root re-exports). Only use explicit paths if needed.

---

### ☐ Step 6.3: Run Tests After Each Update

After updating a test file:

```bash
cargo test --test <test_file_name>
```

---

### ☐ Step 6.4: Run Full Test Suite

```bash
cargo test
```

**Expected:** All 21 tests pass

---

### ☐ Step 6.5: Commit Test Updates

```bash
git add tests/
git commit -m "Refactor: Update test imports for new module structure

- Update imports to use core::, player::, transport::, protocol:: paths
- Verify all 21 tests pass with new structure"
```

---

## Phase 7: Final Verification

### ☐ Step 7.1: Verify All Tests Pass

```bash
cargo test
```

**Expected:** All tests pass

---

### ☐ Step 7.2: Verify no_std Build

```bash
cargo check --no-default-features
```

**Expected:** Success

---

### ☐ Step 7.3: Verify Clippy

```bash
cargo clippy -- -D warnings
```

**Expected:** Zero warnings

---

### ☐ Step 7.4: Verify Release Build

```bash
cargo build --release
```

**Expected:** Success

---

### ☐ Step 7.5: Test Binary Functionality

```bash
# Test local game (AI vs AI)
./target/release/battleship local --player1 ai --player2 ai

# Test --help
./target/release/battleship --help
```

**Expected:** Binary works correctly

---

### ☐ Step 7.6: Verify Git History Preserved

```bash
git log --follow src/core/board.rs
git log --follow src/player/ai.rs
git log --follow src/transport/tcp.rs
```

**Expected:** Full history visible (including commits before the move)

---

### ☐ Step 7.7: Final Commit

```bash
git add -A
git commit -m "Refactor: Complete layered architecture reorganization

Summary of changes:
- Organized src/ into 5 modules: core, player, transport, protocol, cli
- Preserved no_std compatibility for core module
- Maintained backward-compatible public API via root re-exports
- All 21 tests pass
- Zero clippy warnings
- Git history preserved for all moved files

Benefits:
- Clear architectural boundaries (core → player/transport → protocol → cli)
- Easy drop-in replacement for players and transports
- Improved maintainability and navigability
- Foundation for future extensions (web, Bluetooth, hardware)"
```

---

## Phase 8: Update Documentation

### ☐ Step 8.1: Update README.md

Add a section describing the new structure:

```markdown
## Architecture

The repository is organized into layered modules:

- **`core/`**: Pure game logic (no_std compatible)
  - BitBoard, Board, Ship, GameEngine, AI
- **`player/`**: Player trait and implementations
  - AI player, CLI player, PlayerNode orchestrator
- **`transport/`**: Transport trait and implementations
  - In-memory, TCP, Heartbeat wrapper
- **`protocol/`**: Network protocol and RPC framework
  - Message definitions, Skeleton/Stub pattern
- **`cli/`**: Command-line interface
  - CLI commands, interface helpers

### Adding New Components

**New Player Type:**
1. Create `src/player/your_player.rs`
2. Implement `Player` trait
3. Register in `src/player/mod.rs`

**New Transport:**
1. Create `src/transport/your_transport.rs`
2. Implement `Transport` trait
3. Register in `src/transport/mod.rs`

See `docs/planning/reorganization-plan.md` for detailed architecture documentation.
```

---

### ☐ Step 8.2: Update Cargo.toml Documentation

Update package description if needed:

```toml
[package]
description = "Battleship game with modular architecture: drop-in players (AI, CLI, web) and transports (memory, TCP, Bluetooth)"
```

---

### ☐ Step 8.3: Commit Documentation Updates

```bash
git add README.md Cargo.toml
git commit -m "docs: Update documentation for layered architecture"
```

---

## Phase 9: Merge and Cleanup

### ☐ Step 9.1: Push Feature Branch

```bash
git push origin refactor/layered-architecture
```

---

### ☐ Step 9.2: Create Pull Request

Create PR with description:

```markdown
# Refactor: Layered Architecture

## Summary
Reorganizes the flat `src/` directory into a layered architecture with clear module boundaries.

## Structure
- `core/`: Game logic (no_std)
- `player/`: Player trait + implementations
- `transport/`: Transport trait + implementations
- `protocol/`: Network protocol + RPC
- `cli/`: CLI interface

## Benefits
- ✅ Clear architectural boundaries
- ✅ Easy drop-in replacement for components
- ✅ Improved maintainability
- ✅ Foundation for future extensions

## Verification
- All 21 tests pass
- no_std build succeeds
- Zero clippy warnings
- Backward-compatible public API
- Git history preserved

See `docs/planning/reorganization-plan.md` for full design rationale.
```

---

### ☐ Step 9.3: Review and Merge

After review, merge the PR:

```bash
git checkout master
git merge refactor/layered-architecture
git push origin master
```

---

### ☐ Step 9.4: Delete Feature Branch

```bash
git branch -d refactor/layered-architecture
git push origin --delete refactor/layered-architecture
```

---

## Rollback Plan

If issues arise during migration:

### Option 1: Rollback Last Commit

```bash
git reset --hard HEAD~1
```

### Option 2: Rollback to Pre-Migration State

```bash
git checkout master
git branch -D refactor/layered-architecture
```

### Option 3: Cherry-Pick Successful Phases

```bash
# If Phase 1-3 succeeded but Phase 4 failed:
git checkout master
git checkout -b refactor/layered-architecture-v2
git cherry-pick <commit1> <commit2> <commit3>
# Continue from Phase 4
```

---

## Completion Checklist

- [ ] All tests pass (`cargo test`)
- [ ] no_std build succeeds (`cargo check --no-default-features`)
- [ ] Clippy clean (`cargo clippy -- -D warnings`)
- [ ] Release build works (`cargo build --release`)
- [ ] Binary functions correctly
- [ ] Git history preserved
- [ ] Documentation updated
- [ ] PR created and merged

---

## Estimated Timeline

| Phase | Estimated Time |
|-------|---------------|
| Phase 0: Pre-Migration | 15 min |
| Phase 1: Core Module | 30 min |
| Phase 2: Player Module | 30 min |
| Phase 3: Protocol Module | 20 min |
| Phase 4: Transport Module | 10 min |
| Phase 5: CLI Module | 30 min |
| Phase 6: Update Tests | 30 min |
| Phase 7: Verification | 15 min |
| Phase 8: Documentation | 20 min |
| Phase 9: Merge | 10 min |
| **Total** | **3-4 hours** |

---

## Notes

- Take breaks between phases
- Commit after each phase (enables easy rollback)
- Run tests frequently
- If stuck, refer to `reorganization-plan.md` for context
- Ask for help if import errors are confusing

---

**Status Legend:**
- ☐ Not started
- ⧗ In progress
- ✓ Completed
- ✗ Failed (rollback recommended)
