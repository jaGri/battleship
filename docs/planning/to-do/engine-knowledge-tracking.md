# Engine Knowledge Tracking

Improve enemy-ship knowledge so AI and synchronization can reason about sunk ships accurately.

## Current State

- `GameEngine` tracks guess hits, misses, and a total enemy hit counter.
- `active_hits()` currently returns all hits.
- `enemy_ship_lengths_remaining()` currently returns the full fleet.

## Next Questions

- Should the engine track sunk enemy ship identities, lengths, or hit clusters?
- How should protocol messages expose sunk ships without leaking private information?
- How much of this belongs in core versus agent memory?

## Candidate Work

- Track sunk enemy ships from `GuessResult::Sink`.
- Make `active_hits()` exclude hits belonging to sunk ships.
- Make `enemy_ship_lengths_remaining()` return only unsunk enemy ship lengths.
- Add regression tests for AI target selection after sinks.
- Ensure persistence snapshots preserve the new knowledge.
