#[cfg(feature = "std")]
#[cfg(test)]
mod cli_tests {
    use battleship::{CliPlayer, GameEngine, Player};
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn test_cli_player_instantiation() {
        // Just verify CliPlayer can be created
        let _player = CliPlayer::new();
    }

    #[test]
    fn test_cli_with_fixed_seed_placement() {
        // Test that CLI player can use board with fixed RNG seed
        let mut rng = SmallRng::seed_from_u64(12345);
        let mut player = CliPlayer::new();
        let mut engine = GameEngine::new();
        
        // This would require stdin mocking for full test, but we can verify
        // the engine and player are properly initialized
        assert!(engine.board().ship_states().iter().all(|s| !s.sunk));
        
        // Verify the board is empty initially
        let states = engine.board().ship_states();
        assert!(states.iter().all(|s| s.position.is_none()));
    }

    #[test]
    fn test_reproducible_rng() {
        // Test that same seed produces same RNG behavior
        let mut rng1 = SmallRng::seed_from_u64(42);
        let mut rng2 = SmallRng::seed_from_u64(42);
        
        let mut engine1 = GameEngine::new();
        let mut engine2 = GameEngine::new();
        
        // Place ships randomly with same seed
        for i in 0..5 {
            let (r1, c1, o1) = engine1.board_mut().random_placement(&mut rng1, i).unwrap();
            engine1.board_mut().place(i, r1, c1, o1).unwrap();
            
            let (r2, c2, o2) = engine2.board_mut().random_placement(&mut rng2, i).unwrap();
            engine2.board_mut().place(i, r2, c2, o2).unwrap();
            
            // With same seed, placements should be identical
            assert_eq!(r1, r2);
            assert_eq!(c1, c2);
            assert_eq!(o1, o2);
        }
        
        // Verify both boards have same state
        let states1 = engine1.board().ship_states();
        let states2 = engine2.board().ship_states();
        
        for i in 0..5 {
            assert_eq!(states1[i].position, states2[i].position);
        }
    }
}
