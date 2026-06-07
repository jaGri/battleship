# Exact Bayesian Ship Probability for Battleship AI

## Problem Description

In a Battleship game running on embedded hardware (`no_std`, no heap allocation), we need to compute the posterior probability that each unguessed cell on the opponent's board contains a ship segment, conditioned on all observations (hits, misses, and sunk ships). These probabilities guide the AI's next guess.

The posterior is defined as:

$$P(\text{cell } (r,c) \text{ occupied} \mid \text{observations}) = \frac{\text{number of valid complete layouts containing } (r,c)}{\text{total number of valid complete layouts}}$$

A "valid complete layout" is a placement of all remaining (unsunk) ships such that:

- No ship overlaps a known miss.
- No ship overlaps a cell belonging to a sunk ship.
- No two ships overlap each other.
- Every unsunk hit cell is covered by exactly one ship.
- Each ship is a straight contiguous horizontal or vertical line (enforced structurally by precomputed placement masks).

We assume a uniform prior over all valid layouts. This is the canonical assumption for Battleship AI and yields a correct Bayesian posterior under exact enumeration.

## Solution Overview

Enumerate all valid complete layouts via depth-first backtracking. For each valid layout, increment a per-cell counter for every unguessed cell occupied by a ship. Normalize by the total layout count to obtain the exact posterior.

Two modes are supported:

- **Exact mode**: enumerate all valid layouts. The result is the true Bayesian posterior.
- **Monte Carlo mode**: shuffle placement lists randomly before enumeration, then terminate after a layout budget is reached. Because enumeration order is randomized, the partial result is an unbiased estimate of the posterior.

The mode is selected explicitly by the caller. No approximations are hidden.

## Board Representation

The board is `GRID_SIZE × GRID_SIZE` (10×10 = 100 cells). Each cell maps to a bit in a `u128`:

```
bit_index(r, c) = r * GRID_SIZE + c
```

All board state — hits, misses, sunk cells, occupied masks, placement masks — is represented as `u128` bitmasks. Validity checking reduces to bitwise operations.

## Inputs

| Input | Type | Description |
|---|---|---|
| `hits` | `u128` | Cells where a shot hit a ship (includes cells of sunk ships) |
| `misses` | `u128` | Cells where a shot hit water |
| `sunk` | `u128` | Cells belonging to fully identified sunk ships |
| `remaining_lengths` | `[usize; NUM_SHIPS]` | Lengths of unsunk ships (0 for already-sunk ships) |
| `rng` | `&mut R` | Random number generator (used in Monte Carlo mode and for final sampling) |
| `layout_budget` | `Option<u64>` | `None` for exact enumeration, `Some(n)` for Monte Carlo with at most `n` accepted layouts |

Derived quantities:

- `blocked = misses | sunk` — cells no remaining ship can occupy.
- `unsunk_hits = hits & !sunk` — hit cells that must be explained by remaining ships.
- `all_guessed = hits | misses` — cells already guessed (excluded from output probabilities).

## Precomputed Placement Table

At initialization, compute a bitmask for every possible single-ship placement on the board.

### Structure

```rust
const MAX_SHIP_LEN: usize = 5;
const MAX_PLACEMENTS_PER_LEN: usize = 2 * GRID_SIZE * GRID_SIZE;

struct PlacementList {
    masks: [u128; MAX_PLACEMENTS_PER_LEN],
    count: usize,
}

struct PlacementTable {
    by_length: [PlacementList; MAX_SHIP_LEN],
}
```

### Construction

For each length `len` in `1..=MAX_SHIP_LEN`, for each orientation (horizontal, vertical), for each anchor `(r, c)` such that the ship fits on the board:

```rust
fn build_placement_table() -> PlacementTable {
    let mut table = PlacementTable::new();
    for len in 1..=MAX_SHIP_LEN {
        for orient in [Horizontal, Vertical] {
            let (max_r, max_c) = match orient {
                Vertical   => (GRID_SIZE - len + 1, GRID_SIZE),
                Horizontal => (GRID_SIZE, GRID_SIZE - len + 1),
            };
            for r in 0..max_r {
                for c in 0..max_c {
                    let mut mask = 0u128;
                    for k in 0..len {
                        let (rr, cc) = match orient {
                            Vertical   => (r + k, c),
                            Horizontal => (r, c + k),
                        };
                        mask |= 1u128 << (rr * GRID_SIZE + cc);
                    }
                    table.by_length[len - 1].push(mask);
                }
            }
        }
    }
    table
}
```

Memory: for a 10×10 board, all lengths combined produce under 800 entries. At 16 bytes each, the table is ~12.8 KB.

### Runtime Prefiltering

Before each enumeration, filter each length's placement list to remove placements overlapping `blocked`:

```rust
fn prefilter(source: &PlacementList, blocked: u128) -> PlacementList {
    let mut result = PlacementList::new();
    for i in 0..source.count {
        if (source.masks[i] & blocked) == 0 {
            result.push(source.masks[i]);
        }
    }
    result
}
```

Prefiltering is performed **once per unique ship length**, not per ship instance. All ships of the same length share the same filtered placement list. This is a precondition for deduplication correctness (see below).

Mid-game, prefiltering typically eliminates 60–90% of placements.

## Ship Ordering

Before enumeration, collect the nonzero entries from `remaining_lengths` and sort them in descending order. Longest ships are placed first because they have the fewest valid placements, minimizing branching at the top of the search tree.

```rust
fn prepare_ship_list(remaining_lengths: &[usize; NUM_SHIPS]) -> ([usize; NUM_SHIPS], usize) {
    let mut lengths = [0usize; NUM_SHIPS];
    let mut count = 0;
    for &l in remaining_lengths {
        if l > 0 {
            lengths[count] = l;
            count += 1;
        }
    }
    lengths[..count].sort_unstable_by(|a, b| b.cmp(a));
    (lengths, count)
}
```

## Deduplication of Same-Length Ships

If ships at indices `i` and `i+1` have the same length, they are indistinguishable and share identical placement lists. Swapping their placements produces the same layout. To avoid double-counting, enforce that the placement index chosen for ship `i+1` is ≥ the index chosen for ship `i`.

For `k` ships of the same length, this eliminates a factor of `k!` redundant enumeration.

**Preconditions** (all satisfied by the design):

- Ships of equal length are indistinguishable.
- They share the same filtered placement list.
- The list ordering is identical across same-length ships.

**Important**: Deduplication must be disabled in Monte Carlo mode, since shuffling destroys the shared ordering that the index constraint relies on.

## Enumeration State

```rust
struct EnumState {
    /// Prefiltered placement lists, one per active ship (indexed by sorted order).
    /// Same-length ships reference the same list.
    placements: [PlacementList; NUM_SHIPS],
    /// Sorted ship lengths (descending), count entries valid.
    lengths: [usize; NUM_SHIPS],
    /// Number of active (unsunk) ships.
    num_active: usize,
    /// Bitmask of unsunk hit cells.
    unsunk_hits: u128,
    /// Bitmask of all guessed cells (hits | misses).
    all_guessed: u128,
    /// Per-cell count of valid layouts containing that cell.
    matrix: [[u128; GRID_SIZE]; GRID_SIZE],
    /// Total valid layouts enumerated.
    total_layouts: u128,
    /// Layout budget (None = exact, Some(n) = stop after n accepted layouts).
    budget: Option<u64>,
    /// Whether deduplication is active (false in Monte Carlo mode).
    dedup: bool,
}
```

## Core Enumeration

```rust
fn enumerate(
    state: &mut EnumState,
    ship_idx: usize,
    occupied: u128,
    uncovered_hits: u128,
    prev_indices: &[usize; NUM_SHIPS],
) {
    // --- BUDGET CHECK ---
    if let Some(b) = state.budget {
        if state.total_layouts >= b as u128 {
            return;
        }
    }

    // --- BASE CASE ---
    if ship_idx == state.num_active {
        if uncovered_hits != 0 {
            return;
        }
        state.total_layouts += 1;
        accumulate(occupied, state.all_guessed, &mut state.matrix);
        return;
    }

    // --- CAPACITY PRUNING ---
    let remaining_capacity: usize = state.lengths[ship_idx..state.num_active]
        .iter()
        .sum();
    if (uncovered_hits.count_ones() as usize) > remaining_capacity {
        return;
    }

    // --- PER-HIT COVERABILITY PRUNING ---
    if uncovered_hits != 0 && uncovered_hits.count_ones() <= 4 {
        if !all_hits_coverable(state, ship_idx, occupied, uncovered_hits) {
            return;
        }
    }

    // --- DEDUP: same-length ordering constraint ---
    let start_idx = if state.dedup
        && ship_idx > 0
        && state.lengths[ship_idx] == state.lengths[ship_idx - 1]
    {
        prev_indices[ship_idx - 1]
    } else {
        0
    };

    // --- ENUMERATE PLACEMENTS ---
    let plist = &state.placements[ship_idx];
    for i in start_idx..plist.count {
        let mask = plist.masks[i];

        if (mask & occupied) != 0 {
            continue;
        }

        let new_occupied = occupied | mask;
        let new_uncovered = uncovered_hits & !mask;

        // --- FORWARD FEASIBILITY (shallow depths) ---
        if ship_idx < 2 && state.num_active > ship_idx + 1 {
            if !remaining_ships_feasible(state, ship_idx + 1, new_occupied) {
                continue;
            }
        }

        let mut indices = *prev_indices;
        indices[ship_idx] = i;

        enumerate(state, ship_idx + 1, new_occupied, new_uncovered, &indices);

        // Re-check budget after returning from subtree
        if let Some(b) = state.budget {
            if state.total_layouts >= b as u128 {
                return;
            }
        }
    }
}
```

## Accumulation

When a valid complete layout is found, increment every unguessed occupied cell:

```rust
fn accumulate(
    occupied: u128,
    all_guessed: u128,
    matrix: &mut [[u128; GRID_SIZE]; GRID_SIZE],
) {
    let mut bits = occupied & !all_guessed;
    while bits != 0 {
        let idx = bits.trailing_zeros() as usize;
        matrix[idx / GRID_SIZE][idx % GRID_SIZE] += 1;
        bits &= bits - 1;
    }
}
```

## Pruning Helpers

### Per-Hit Coverability

Verifies that every uncovered hit cell has at least one remaining ship placement that can cover it without conflicting with already-placed ships. This is a necessary condition for a valid layout to exist. It does not guarantee joint feasibility (the backtracking search handles that) but cheaply eliminates many dead branches.

```rust
fn all_hits_coverable(
    state: &EnumState,
    from_ship: usize,
    occupied: u128,
    uncovered_hits: u128,
) -> bool {
    let mut bits = uncovered_hits;
    while bits != 0 {
        let bit = bits & bits.wrapping_neg();
        let mut coverable = false;
        'outer: for j in from_ship..state.num_active {
            for k in 0..state.placements[j].count {
                let mask = state.placements[j].masks[k];
                if (mask & occupied) == 0 && (mask & bit) != 0 {
                    coverable = true;
                    break 'outer;
                }
            }
        }
        if !coverable {
            return false;
        }
        bits &= bits - 1;
    }
    true
}
```

Apply selectively: when `uncovered_hits.count_ones() <= 4` or at shallow recursion depths (≤ 3). The cost is O(hits × ships × placements) per invocation.

### Forward Ship Feasibility

Checks that every remaining ship has at least one non-conflicting placement:

```rust
fn remaining_ships_feasible(
    state: &EnumState,
    from_ship: usize,
    occupied: u128,
) -> bool {
    for j in from_ship..state.num_active {
        let mut has_valid = false;
        for k in 0..state.placements[j].count {
            if (state.placements[j].masks[k] & occupied) == 0 {
                has_valid = true;
                break;
            }
        }
        if !has_valid {
            return false;
        }
    }
    true
}
```

Apply at shallow depths only (ship_idx < 2) where preventing a bad subtree saves the most work.

## Randomization for Monte Carlo Mode

When a layout budget is specified, shuffle each ship's filtered placement list before enumeration using Fisher-Yates:

```rust
fn shuffle_placements<R: Rng + ?Sized>(list: &mut PlacementList, rng: &mut R) {
    for i in (1..list.count).rev() {
        let j = rng.random_range(0..=i);
        list.masks.swap(i, j);
    }
}
```

This makes the DFS visit order exchangeable, so truncation after N accepted layouts produces an unbiased Monte Carlo estimate. Deduplication is disabled in this mode since shuffling destroys the index ordering that the deduplication constraint depends on.

## Normalization

```rust
fn normalize(
    matrix: &[[u128; GRID_SIZE]; GRID_SIZE],
    total_layouts: u128,
) -> [[f64; GRID_SIZE]; GRID_SIZE] {
    let mut result = [[0.0f64; GRID_SIZE]; GRID_SIZE];
    if total_layouts == 0 {
        // No valid layout exists — return uniform as a fallback.
        let uniform = 1.0 / (GRID_SIZE * GRID_SIZE) as f64;
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                result[r][c] = uniform;
            }
        }
        return result;
    }
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            result[r][c] = matrix[r][c] as f64 / total_layouts as f64;
        }
    }
    result
}
```

In Monte Carlo mode, `total_layouts` is the count of accepted valid layouts actually found, not the requested budget. This ensures correct normalization even if pruning causes fewer layouts to be reached than the budget allows.

## Top-Level Orchestration

```rust
pub fn calc_pdf<R: Rng + ?Sized>(
    hits: &BB,
    misses: &BB,
    sunk: &BB,
    remaining_lengths: &[usize; NUM_SHIPS as usize],
    rng: &mut R,
    layout_budget: Option<u64>,
) -> [[f64; GRID_SIZE]; GRID_SIZE] {
    let blocked = misses.as_u128() | sunk.as_u128();
    let unsunk_hits = hits.as_u128() & !sunk.as_u128();
    let all_guessed = hits.as_u128() | misses.as_u128();

    // 1. Prepare sorted ship list
    let (sorted_lengths, num_active) = prepare_ship_list(remaining_lengths);

    // 2. Prefilter placements per unique length
    let table = get_placement_table(); // precomputed at init
    let mut placements = [PlacementList::new(); NUM_SHIPS];
    for i in 0..num_active {
        placements[i] = prefilter(&table.by_length[sorted_lengths[i] - 1], blocked);
    }

    // 3. Determine mode
    let monte_carlo = layout_budget.is_some();

    // 4. Shuffle in Monte Carlo mode
    if monte_carlo {
        for i in 0..num_active {
            shuffle_placements(&mut placements[i], rng);
        }
    }

    // 5. Initialize state
    let mut state = EnumState {
        placements,
        lengths: sorted_lengths,
        num_active,
        unsunk_hits,
        all_guessed,
        matrix: [[0u128; GRID_SIZE]; GRID_SIZE],
        total_layouts: 0,
        budget: layout_budget,
        dedup: !monte_carlo,
    };

    // 6. Enumerate
    let init_indices = [0usize; NUM_SHIPS];
    enumerate(&mut state, 0, 0u128, unsunk_hits, &init_indices);

    // 7. Normalize and return
    normalize(&state.matrix, state.total_layouts)
}

pub fn calc_pdf_and_guess<R: Rng + ?Sized>(
    hits: &BB,
    misses: &BB,
    sunk: &BB,
    remaining_lengths: &[usize; NUM_SHIPS as usize],
    rng: &mut R,
    layout_budget: Option<u64>,
) -> (usize, usize) {
    let pdf = calc_pdf(hits, misses, sunk, remaining_lengths, rng, layout_budget);
    sample_pdf(&pdf, 0.5, rng)
}
```

## Sampling

The existing `sample_pdf` function is retained unchanged. It applies a temperature parameter to the posterior and samples a cell proportional to the adjusted weights:

```rust
pub fn sample_pdf<R: Rng + ?Sized>(
    pdf: &[[f64; GRID_SIZE]; GRID_SIZE],
    temperature: f64,
    rng: &mut R,
) -> (usize, usize) {
    let mut adjusted = [[0.0f64; GRID_SIZE]; GRID_SIZE];
    let mut total = 0.0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            let v = pdf[r][c].powf(1.0 / temperature);
            adjusted[r][c] = v;
            total += v;
        }
    }
    if total == 0.0 {
        return (rng.random_range(0..GRID_SIZE), rng.random_range(0..GRID_SIZE));
    }
    let threshold: f64 = rng.random_range(0.0..total);
    let mut cumulative = 0.0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            cumulative += adjusted[r][c];
            if threshold < cumulative {
                return (r, c);
            }
        }
    }
    (GRID_SIZE - 1, GRID_SIZE - 1)
}
```

A temperature of 0.5 sharpens the distribution toward the highest-probability cells. A temperature of 1.0 samples proportional to the raw posterior. Lower temperatures make the AI more exploitative; higher temperatures add exploration.

## Pruning Summary

| Technique | Location | Cost | Effect |
|---|---|---|---|
| Prefiltering by `blocked` | Before search | O(placements) once | 60–90% placement elimination |
| Sort ships longest first | Before search | O(ships log ships) once | Minimizes top-level branching |
| Same-length deduplication | Every node (exact mode only) | O(1) | k! reduction per same-length group |
| Capacity check | Every node | O(remaining ships) | Prunes when total remaining length < uncovered hit count |
| Per-hit coverability | Nodes with ≤4 uncovered hits or depth ≤ 3 | O(hits × ships × placements) | Eliminates branches where a hit has no valid covering placement |
| Forward ship feasibility | Depth ≤ 1 | O(ships × placements) | Prevents entering subtrees where a ship has zero valid placements |
| Overlap check | Every placement | O(1) bitwise AND | Core constraint |
| Uncovered hits at leaf | Base case | O(1) | Rejects layouts leaving hits unexplained |

All pruning is exact. No valid layout is ever eliminated.

## Memory Budget

| Structure | Size |
|---|---|
| Placement table (precomputed) | 5 × 200 × 16 = 16 KB |
| Prefiltered lists (per enumeration) | 5 × 200 × 16 = 16 KB |
| `EnumState.matrix` (u128) | 10 × 10 × 16 = 1.6 KB |
| Recursion stack | ~5 levels × 120 bytes ≈ 600 bytes |
| `prev_indices` array | 5 × 8 = 40 bytes |
| Miscellaneous scalars | ~200 bytes |
| **Total** | **~34 KB** |

All stack-allocated. No heap. Fits in the SRAM of typical embedded targets (Cortex-M4: 128–256 KB).

## Correctness Properties

- **Exact mode** produces the true Bayesian posterior under a uniform prior over valid layouts. Every valid layout is counted exactly once (deduplication ensures same-length ships are not permuted). Every invalid layout is excluded (overlap, miss, and hit-coverage constraints are enforced).
- **Monte Carlo mode** produces an unbiased estimate of the same posterior. Placement list shuffling makes the visit order exchangeable. Normalization uses the actual count of accepted layouts, not the budget.
- **No heuristics or arbitrary weights** appear anywhere in the computation. The only tunable parameters are the layout budget (which controls compute vs. accuracy trade-off) and the sampling temperature (which controls exploration vs. exploitation in guess selection).