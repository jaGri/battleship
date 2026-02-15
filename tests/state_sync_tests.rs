use battleship::protocol::GameApi;
use battleship::{
    BitBoard, BoardState, GameEngine, GameState, GuessBoardState, ShipState,
    BOARD_SIZE, NUM_SHIPS, TOTAL_SHIP_CELLS,
};
use battleship::domain::SyncPayload;

#[tokio::test]
async fn test_sync_state_restores_enemy_ships_remaining() {
    // Create an engine and set up a scenario where some ships are sunk
    let mut engine = GameEngine::new();
    
    // Record a hit
    engine.record_guess(0, 0, battleship::GuessResult::Hit).unwrap();
    
    // Record sinking of Destroyer (2 cells) - this is the final hit that sinks it
    engine.record_guess(1, 0, battleship::GuessResult::Sink("Destroyer")).unwrap();
    
    // Verify initial state
    assert_eq!(engine.enemy_ship_lengths_remaining()[4], 0); // Destroyer is sunk (index 4)
    assert_eq!(engine.enemy_ship_lengths_remaining()[0], 5); // Carrier still afloat (index 0)
    
    // Get the current state - we made 2 guesses, both hit
    let state = engine.state();
    assert_eq!(state.enemy_remaining, TOTAL_SHIP_CELLS - 2); // 2 hits total
    assert!(!state.enemy_ships_remaining[4]); // Destroyer sunk
    assert!(state.enemy_ships_remaining[0]); // Carrier still afloat
    
    // Create a new engine and sync it with the state
    let mut engine2 = GameEngine::new();
    let sync_payload = SyncPayload {
        game_state: state,
        enemy_ships_remaining: state.enemy_ships_remaining,
    };
    
    engine2.sync_state(sync_payload).await.unwrap();
    
    // Verify that enemy_ships_remaining was correctly restored
    assert_eq!(engine2.enemy_ship_lengths_remaining()[4], 0); // Destroyer should still be sunk
    assert_eq!(engine2.enemy_ship_lengths_remaining()[0], 5); // Carrier should still be afloat
    
    // Verify the state matches
    let state2 = engine2.state();
    assert_eq!(state2.enemy_remaining, state.enemy_remaining);
    assert_eq!(state2.enemy_ships_remaining, state.enemy_ships_remaining);
}

#[tokio::test]
async fn test_from_state_preserves_enemy_ships_remaining() {
    // Create a game state with mixed ship status
    let mut enemy_ships = [true; NUM_SHIPS as usize];
    enemy_ships[1] = false; // Battleship sunk
    enemy_ships[3] = false; // Submarine sunk
    
    let state = GameState {
        my_board: BoardState {
            ship_states: [
                ShipState::new("Carrier"),
                ShipState::new("Battleship"),
                ShipState::new("Cruiser"),
                ShipState::new("Submarine"),
                ShipState::new("Destroyer"),
            ],
            ship_map: BitBoard::<u128, { BOARD_SIZE as usize }>::new(),
            hits: BitBoard::<u128, { BOARD_SIZE as usize }>::new(),
            misses: BitBoard::<u128, { BOARD_SIZE as usize }>::new(),
        },
        my_guesses: GuessBoardState {
            hits: BitBoard::<u128, { BOARD_SIZE as usize }>::new(),
            misses: BitBoard::<u128, { BOARD_SIZE as usize }>::new(),
        },
        enemy_ships_remaining: enemy_ships,
        enemy_remaining: TOTAL_SHIP_CELLS - 7, // Battleship (4) + Submarine (3) = 7 cells
    };
    
    // Restore engine from state
    let engine = GameEngine::from_state(state);
    
    // Verify enemy_ships_remaining was properly restored (not reset to all true)
    let lengths = engine.enemy_ship_lengths_remaining();
    assert_eq!(lengths[0], 5); // Carrier: afloat
    assert_eq!(lengths[1], 0); // Battleship: sunk
    assert_eq!(lengths[2], 3); // Cruiser: afloat
    assert_eq!(lengths[3], 0); // Submarine: sunk
    assert_eq!(lengths[4], 2); // Destroyer: afloat
    
    // Verify the state round-trips correctly
    let restored_state = engine.state();
    assert_eq!(restored_state.enemy_ships_remaining, enemy_ships);
    assert_eq!(restored_state.enemy_remaining, TOTAL_SHIP_CELLS - 7);
}

#[tokio::test]
async fn test_sync_state_full_roundtrip() {
    // Create an engine with a complex state
    let mut engine1 = GameEngine::new();
    
    // Make some guesses and record results
    engine1.record_guess(0, 0, battleship::GuessResult::Hit).unwrap();
    engine1.record_guess(0, 1, battleship::GuessResult::Miss).unwrap();
    engine1.record_guess(1, 0, battleship::GuessResult::Hit).unwrap();
    engine1.record_guess(1, 1, battleship::GuessResult::Sink("Destroyer")).unwrap();
    engine1.record_guess(2, 2, battleship::GuessResult::Hit).unwrap();
    
    // Get initial state
    let state1 = engine1.state();
    
    // Create sync payload
    let sync_payload = SyncPayload {
        game_state: state1,
        enemy_ships_remaining: state1.enemy_ships_remaining,
    };
    
    // Create a new engine and sync
    let mut engine2 = GameEngine::new();
    engine2.sync_state(sync_payload).await.unwrap();
    
    // Verify the states match exactly
    let state2 = engine2.state();
    assert_eq!(state1.enemy_remaining, state2.enemy_remaining);
    assert_eq!(state1.enemy_ships_remaining, state2.enemy_ships_remaining);
    assert_eq!(state1.my_guesses.hits, state2.my_guesses.hits);
    assert_eq!(state1.my_guesses.misses, state2.my_guesses.misses);
    
    // Verify ship lengths match
    let lengths1 = engine1.enemy_ship_lengths_remaining();
    let lengths2 = engine2.enemy_ship_lengths_remaining();
    assert_eq!(lengths1, lengths2);
}
