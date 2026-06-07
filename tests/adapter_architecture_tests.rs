#![cfg(feature = "std")]

use battleship::transport::TransportEndpoint;
use battleship::{
    AiAgent, AiDifficulty, AppCommand, AppEvent, BattleshipApp, InMemoryTransport, MemorySaveStore,
    SaveStore, ScreenView, WireMessage, PROTOCOL_VERSION,
};
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[test]
fn app_generates_game_screen_view() {
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Medium),
        AiAgent::new(AiDifficulty::Medium),
    );

    match app.view() {
        ScreenView::Game(game) => {
            assert!(game.my_turn);
            assert_eq!(game.turn_number, 0);
        }
        _ => panic!("expected game view"),
    }
}

#[test]
fn app_loaded_event_restores_saved_game() {
    let mut rng1 = SmallRng::seed_from_u64(1);
    let mut rng2 = SmallRng::seed_from_u64(2);
    let mut app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Easy),
    );
    app.place_ships(&mut rng1, &mut rng2).unwrap();
    app.play_next_turn(&mut rng1, &mut rng2).unwrap();
    let saved = app.match_state.saved_game();

    let mut restored = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Easy),
    );
    let commands = restored.update(AppEvent::Loaded(Some(saved)));

    assert!(commands.iter().any(|cmd| matches!(cmd, AppCommand::Render)));
    assert_eq!(restored.match_state.turn_number, saved.turn_number);
}

#[test]
fn memory_save_store_round_trips_active_game() {
    let mut store = MemorySaveStore::default();
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );
    let saved = app.match_state.saved_game();

    store.save_active(&saved).unwrap();
    assert_eq!(store.load_active().unwrap(), Some(saved));
    store.clear_active().unwrap();
    assert_eq!(store.load_active().unwrap(), None);
}

#[test]
fn in_memory_endpoint_sends_wire_messages_without_awaiting() {
    let (mut left, mut right) = InMemoryTransport::pair();
    let msg = WireMessage::Heartbeat {
        version: PROTOCOL_VERSION,
    };

    TransportEndpoint::send(&mut left, &msg).unwrap();
    assert_eq!(right.poll().unwrap(), Some(msg));
    assert!(left.is_connected());
}
