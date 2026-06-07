#![cfg(feature = "std")]

#[cfg(feature = "in-memory")]
use battleship::transport::TransportEndpoint;
#[cfg(feature = "in-memory")]
use battleship::InMemoryTransport;
use battleship::{AiAgent, AiDifficulty, AppCommand, AppEvent, BattleshipApp, ScreenView, UiEvent};
#[cfg(feature = "persistence")]
use battleship::{FileSaveError, SaveIntegrityError};
#[cfg(feature = "persistence")]
use battleship::{FileSaveStore, MemorySaveStore, SaveStore};
#[cfg(feature = "in-memory")]
use battleship::{WireMessage, PROTOCOL_VERSION};
use rand::rngs::SmallRng;
use rand::SeedableRng;
#[cfg(feature = "persistence")]
use std::path::PathBuf;
#[cfg(feature = "persistence")]
use std::{fs, io};

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
fn file_save_store_rejects_tampered_payload() {
    let path = test_save_path("tampered-payload");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );

    store.save_active(&app.match_state.saved_game()).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    let last = bytes.last_mut().unwrap();
    *last ^= 0x01;
    fs::write(&path, bytes).unwrap();

    let err = store.load_active().unwrap_err();
    assert!(matches!(
        err,
        FileSaveError::Integrity(SaveIntegrityError::Mac)
    ));
    cleanup_save_path(&path);
}

#[test]
#[cfg(feature = "persistence")]
fn file_save_store_rejects_tampered_header() {
    let path = test_save_path("tampered-header");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );

    store.save_active(&app.match_state.saved_game()).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[0] ^= 0x01;
    fs::write(&path, bytes).unwrap();

    let err = store.load_active().unwrap_err();
    assert!(matches!(
        err,
        FileSaveError::Integrity(SaveIntegrityError::Header)
    ));
    cleanup_save_path(&path);
}

#[test]
#[cfg(feature = "persistence")]
fn file_save_store_rejects_unsupported_save_version() {
    let path = test_save_path("tampered-version");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );

    store.save_active(&app.match_state.saved_game()).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[4] = bytes[4].wrapping_add(1);
    fs::write(&path, bytes).unwrap();

    let err = store.load_active().unwrap_err();
    assert!(matches!(
        err,
        FileSaveError::Integrity(SaveIntegrityError::Version)
    ));
    cleanup_save_path(&path);
}

#[test]
#[cfg(feature = "persistence")]
fn file_save_store_rejects_legacy_plain_bincode_save() {
    let path = test_save_path("legacy-bincode");
    let mut store = FileSaveStore::new(&path);
    store.clear_active().unwrap();
    let app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );
    let legacy_bytes = bincode::serialize(&app.match_state.saved_game()).unwrap();
    fs::write(&path, legacy_bytes).unwrap();

    let err = store.load_active().unwrap_err();
    assert!(matches!(
        err,
        FileSaveError::Integrity(SaveIntegrityError::Header)
    ));
    cleanup_save_path(&path);
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

#[cfg(feature = "persistence")]
fn cleanup_save_path(path: &PathBuf) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => panic!("failed to remove test save file: {err}"),
    }
}
