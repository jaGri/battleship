use battleship::{GameEngine, GuessResult as CommonGuessResult, BOARD_SIZE, NUM_SHIPS};
use proptest::prelude::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};

fn random_engine(seed: u64) -> GameEngine {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut engine = GameEngine::new();
    
    // Place ships on board
    for i in 0..NUM_SHIPS as usize {
        let (r, c, orient) = engine.board_mut().random_placement(&mut rng, i).unwrap();
        engine.board_mut().place(i, r, c, orient).unwrap();
    }
    
    // Make some random guesses on our board
    let my_guesses = rng.random_range(0..(BOARD_SIZE as usize * BOARD_SIZE as usize / 4));
    for _ in 0..my_guesses {
        let r = rng.random_range(0..BOARD_SIZE as usize);
        let c = rng.random_range(0..BOARD_SIZE as usize);
        let _ = engine.opponent_guess(r, c);
    }
    
    // Record some guesses against opponent
    let opponent_guesses = rng.random_range(0..(BOARD_SIZE as usize * BOARD_SIZE as usize / 4));
    for _ in 0..opponent_guesses {
        let r = rng.random_range(0..BOARD_SIZE as usize);
        let c = rng.random_range(0..BOARD_SIZE as usize);
        // Skip if already guessed
        if engine.guess_hits().get(r, c).unwrap_or(false) || 
           engine.guess_misses().get(r, c).unwrap_or(false) {
            continue;
        }
        // Randomly decide hit or miss
        let result = if rng.random_bool(0.3) {
            CommonGuessResult::Hit
        } else {
            CommonGuessResult::Miss
        };
        let _ = engine.record_guess(r, c, result);
    }
    
    engine
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that GameEngine::state() followed by GameEngine::from_state() preserves all game state
    #[test]
    fn game_engine_state_roundtrip(seed in any::<u64>()) {
        let engine1 = random_engine(seed);
        let state = engine1.state();
        let engine2 = GameEngine::from_state(state);
        let state2 = engine2.state();
        
        // States should be identical
        prop_assert_eq!(state.my_board, state2.my_board);
        prop_assert_eq!(state.my_guesses.hits, state2.my_guesses.hits);
        prop_assert_eq!(state.my_guesses.misses, state2.my_guesses.misses);
        prop_assert_eq!(state.enemy_ships_remaining, state2.enemy_ships_remaining);
        prop_assert_eq!(state.enemy_remaining, state2.enemy_remaining);
        
        // Boards should be identical
        prop_assert_eq!(engine1.board().ship_states(), engine2.board().ship_states());
        
        // Guess bitboards should match
        prop_assert_eq!(engine1.guess_hits(), engine2.guess_hits());
        prop_assert_eq!(engine1.guess_misses(), engine2.guess_misses());
    }

    /// Test that record_guess properly updates state and that it round-trips correctly
    #[test]
    fn record_guess_roundtrip(
        seed in any::<u64>(),
        row in 0..BOARD_SIZE as usize,
        col in 0..BOARD_SIZE as usize,
        is_hit in any::<bool>()
    ) {
        let mut engine = random_engine(seed);
        
        // Skip if already guessed
        if engine.guess_hits().get(row, col).unwrap_or(false) || 
           engine.guess_misses().get(row, col).unwrap_or(false) {
            return Ok(());
        }
        
        let result = if is_hit {
            CommonGuessResult::Hit
        } else {
            CommonGuessResult::Miss
        };
        
        let _state_before = engine.state();
        engine.record_guess(row, col, result).unwrap();
        let state_after = engine.state();
        
        // Verify the guess was recorded
        if is_hit {
            prop_assert!(engine.guess_hits().get(row, col).unwrap());
        } else {
            prop_assert!(engine.guess_misses().get(row, col).unwrap());
        }
        
        // Round-trip the state
        let engine_restored = GameEngine::from_state(state_after);
        let state_restored = engine_restored.state();
        
        prop_assert_eq!(state_after.my_board, state_restored.my_board);
        prop_assert_eq!(state_after.my_guesses.hits, state_restored.my_guesses.hits);
        prop_assert_eq!(state_after.my_guesses.misses, state_restored.my_guesses.misses);
        prop_assert_eq!(state_after.enemy_ships_remaining, state_restored.enemy_ships_remaining);
        prop_assert_eq!(state_after.enemy_remaining, state_restored.enemy_remaining);
    }

    /// Test that recording sunk ships properly updates enemy_ships_remaining
    #[test]
    fn record_sink_roundtrip(seed in any::<u64>(), ship_idx in 0..NUM_SHIPS as usize) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut engine = GameEngine::new();
        
        // Place ships
        for i in 0..NUM_SHIPS as usize {
            let (r, c, orient) = engine.board_mut().random_placement(&mut rng, i).unwrap();
            engine.board_mut().place(i, r, c, orient).unwrap();
        }
        
        // Find a valid cell to guess
        let row = rng.random_range(0..BOARD_SIZE as usize);
        let col = rng.random_range(0..BOARD_SIZE as usize);
        
        if engine.guess_hits().get(row, col).unwrap_or(false) || 
           engine.guess_misses().get(row, col).unwrap_or(false) {
            return Ok(());
        }
        
        // Get the ship name from config
        let ship_name = battleship::SHIPS[ship_idx].name();
        
        // Record a sink
        let result = CommonGuessResult::Sink(ship_name);
        engine.record_guess(row, col, result).unwrap();
        
        // Verify ship is marked as sunk
        prop_assert_eq!(engine.state().enemy_ships_remaining[ship_idx], false);
        
        // Round-trip
        let state = engine.state();
        let engine2 = GameEngine::from_state(state);
        prop_assert_eq!(engine2.state().enemy_ships_remaining[ship_idx], false);
    }

    /// Test multiple guesses and verify state consistency after round-trip
    #[test]
    fn multiple_guesses_roundtrip(seed in any::<u64>(), num_guesses in 1..20usize) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut engine = GameEngine::new();
        
        // Place ships
        for i in 0..NUM_SHIPS as usize {
            let (r, c, orient) = engine.board_mut().random_placement(&mut rng, i).unwrap();
            engine.board_mut().place(i, r, c, orient).unwrap();
        }
        
        // Make multiple guesses
        for _ in 0..num_guesses {
            let r = rng.random_range(0..BOARD_SIZE as usize);
            let c = rng.random_range(0..BOARD_SIZE as usize);
            
            // Skip if already guessed
            if engine.guess_hits().get(r, c).unwrap_or(false) || 
               engine.guess_misses().get(r, c).unwrap_or(false) {
                continue;
            }
            
            let result = if rng.random_bool(0.4) {
                CommonGuessResult::Hit
            } else {
                CommonGuessResult::Miss
            };
            
            let _ = engine.record_guess(r, c, result);
        }
        
        // Round-trip
        let state1 = engine.state();
        let engine2 = GameEngine::from_state(state1);
        let state2 = engine2.state();
        
        prop_assert_eq!(state1.my_guesses.hits, state2.my_guesses.hits);
        prop_assert_eq!(state1.my_guesses.misses, state2.my_guesses.misses);
        prop_assert_eq!(state1.enemy_remaining, state2.enemy_remaining);
        
        // Verify guess counts by checking each cell
        let mut hits1 = 0;
        let mut hits2 = 0;
        for r in 0..BOARD_SIZE as usize {
            for c in 0..BOARD_SIZE as usize {
                if engine.guess_hits().get(r, c).unwrap_or(false) {
                    hits1 += 1;
                }
                if engine2.guess_hits().get(r, c).unwrap_or(false) {
                    hits2 += 1;
                }
            }
        }
        prop_assert_eq!(hits1, hits2);
    }
}
