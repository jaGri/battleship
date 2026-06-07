#![cfg(feature = "std")]

#[cfg(feature = "in-memory")]
use battleship::transport::TransportEndpoint;
#[cfg(feature = "in-memory")]
use battleship::InMemoryTransport;
use battleship::{AiAgent, AiDifficulty, AppCommand, AppEvent, BattleshipApp, ScreenView, UiEvent};
#[cfg(feature = "persistence")]
use battleship::{FileSaveStore, MemorySaveStore, SaveStore};
#[cfg(feature = "in-memory")]
use battleship::{WireMessage, PROTOCOL_VERSION};
use rand::rngs::SmallRng;
use rand::SeedableRng;
#[cfg(feature = "persistence")]
use std::path::PathBuf;

#[test]
fn app_generates_game_screen_view() {
    let mut app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Medium),
        AiAgent::new(AiDifficulty::Medium),
    );
    app.update(AppEvent::Ui(UiEvent::Start));

    match app.view() {
        ScreenView::Menu(menu) => {
            assert_eq!(menu.title, "Battleship");
            assert_eq!(menu.selected, 0);
        }
        _ => panic!("expected menu view"),
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
#[cfg(feature = "persistence")]
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
#[cfg(feature = "persistence")]
fn file_save_store_round_trips_active_game() {
    let path = test_save_path("round-trip");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();
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
#[cfg(feature = "persistence")]
fn file_save_store_loads_missing_file_as_empty() {
    let path = test_save_path("missing");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();

    assert_eq!(store.load_active().unwrap(), None);
}

#[test]
#[cfg(feature = "persistence")]
fn file_save_store_clears_missing_file() {
    let path = test_save_path("clear-missing");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();

    store.clear_active().unwrap();
    assert_eq!(store.load_active().unwrap(), None);
}

#[test]
#[cfg(feature = "in-memory")]
fn in_memory_endpoint_sends_wire_messages_without_awaiting() {
    let (mut left, mut right) = InMemoryTransport::pair();
    let msg = WireMessage::Heartbeat {
        version: PROTOCOL_VERSION,
    };

    TransportEndpoint::send(&mut left, &msg).unwrap();
    assert_eq!(right.poll().unwrap(), Some(msg));
    assert!(left.is_connected());
}

#[cfg(feature = "persistence")]
fn test_save_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "battleship-{name}-{}-{nanos}.sav",
        std::process::id(),
    ))
}
