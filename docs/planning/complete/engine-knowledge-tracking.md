# Engine Knowledge Tracking

Completed enemy ship footprint tracking so AI, rendering, persistence, and
remote synchronization can reason about sunk ships accurately.

## Completion Summary

- Replaced the split `Ship`/`ShipState` model with one reusable core `Ship`
  state that carries name, length, sunk status, hits, known placement, and
  optional position.
- Changed `Board` and `BoardState` to use fixed ship arrays directly.
- Changed `GameEngine` to track enemy ships with the same `Ship` type.
- Made `active_hits()` exclude hits that belong to known sunk enemy footprints.
- Made `enemy_ship_lengths_remaining()` derive from tracked enemy ship state.
- Added fixed-bitboard sink footprints to protocol sink responses and bumped
  the protocol version.
- Kept core footprint tracking allocation-free.
- Updated save/load and private sync snapshots to preserve enemy ship footprint
  state.
- Updated AI targeting so restored sunk footprints do not keep boosting target
  probabilities.

## Verification

- `cargo fmt`
- `cargo check --no-default-features`
- `cargo check --all-features`
- `cargo test --all-features`
- `cargo clippy --all-features -- -D warnings`

## Resolved Questions

- Sunk enemy ships are represented in core with the same `Ship` state used for
  owned ships.
- Protocol sink results expose a fixed `BitBoard` footprint, not heap-backed
  coordinate lists.
- Core owns persisted sunk footprint state; agents may keep memory, but restored
  engine state remains authoritative for active-hit behavior.
