use battleship::{GameEngine, GameState, NUM_SHIPS};
use proptest::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;

proptest! {
    #[test]
    fn game_state_roundtrip(seed in any::<u64>()) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut engine = GameEngine::new();
        for i in 0..NUM_SHIPS as usize {
            let (r, c, o) = engine.board().random_placement(&mut rng, i).unwrap();
            engine.board_mut().place(i, r, c, o).unwrap();
        }
        let state = engine.state();
        let bytes = bincode::serialize(&state).unwrap();
        let decoded: GameState = bincode::deserialize(&bytes).unwrap();
        let restored = GameEngine::from_state(decoded);
        assert_eq!(engine.state(), restored.state());
    }
}
