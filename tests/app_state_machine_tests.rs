#![cfg(feature = "std")]

use battleship::domain::RemotePlayer;
use battleship::{
    AgentAction, AgentPromptKind, AiDifficulty, AppCommand, AppEvent, AppState, BattleshipApp,
    BitBoard, ConnectionStatus, GuessBoardState, MatchMode, Orientation, PlacementMode, PlayerSide,
    RemoteRole, SavedGame, ScriptedAgent, ShipPlacement, UiEvent, WireMessage, PROTOCOL_VERSION,
};

fn placements() -> AgentAction {
    AgentAction::PlaceShips(vec![
        ShipPlacement {
            ship_index: 0,
            row: 0,
            col: 0,
            orientation: Orientation::Horizontal,
        },
        ShipPlacement {
            ship_index: 1,
            row: 1,
            col: 0,
            orientation: Orientation::Horizontal,
        },
        ShipPlacement {
            ship_index: 2,
            row: 2,
            col: 0,
            orientation: Orientation::Horizontal,
        },
        ShipPlacement {
            ship_index: 3,
            row: 3,
            col: 0,
            orientation: Orientation::Horizontal,
        },
        ShipPlacement {
            ship_index: 4,
            row: 4,
            col: 0,
            orientation: Orientation::Horizontal,
        },
    ])
}

fn new_app() -> BattleshipApp<ScriptedAgent, ScriptedAgent> {
    BattleshipApp::new_local_ai(ScriptedAgent::default(), ScriptedAgent::default())
}

fn setup_solo(app: &mut BattleshipApp<ScriptedAgent, ScriptedAgent>) -> Vec<AppCommand> {
    app.update(AppEvent::Ui(UiEvent::Start));
    app.update(AppEvent::Ui(UiEvent::Confirm));
    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));
    assert!(matches!(
        commands.as_slice(),
        [AppCommand::RequestAgent(_), AppCommand::Render]
    ));
    app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    app.update(AppEvent::Agent {
        side: PlayerSide::Opponent,
        action: placements(),
    })
}

fn ship_targets() -> Vec<(usize, usize)> {
    vec![
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (2, 2),
        (3, 0),
        (3, 1),
        (3, 2),
        (4, 0),
        (4, 1),
    ]
}

fn safe_misses() -> Vec<(usize, usize)> {
    vec![
        (9, 9),
        (9, 8),
        (9, 7),
        (9, 6),
        (9, 5),
        (9, 4),
        (9, 3),
        (9, 2),
        (9, 1),
        (9, 0),
        (8, 9),
        (8, 8),
        (8, 7),
        (8, 6),
        (8, 5),
        (8, 4),
    ]
}

#[test]
fn title_menu_and_solo_setup_request_placements() {
    let mut app = new_app();

    assert_eq!(app.state, AppState::Title);
    app.update(AppEvent::Ui(UiEvent::Start));
    assert_eq!(app.state, AppState::MainMenu);

    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));
    assert_eq!(app.state, AppState::SoloSetup);
    assert_eq!(commands, vec![AppCommand::Render]);

    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));
    assert!(matches!(
        commands.as_slice(),
        [AppCommand::RequestAgent(prompt), AppCommand::Render]
            if prompt.side == PlayerSide::Local
                && prompt.kind == AgentPromptKind::PlaceShips(PlacementMode::Random)
    ));
}

#[test]
fn solo_setup_manual_selection_requests_manual_placement() {
    let mut app = new_app();
    app.update(AppEvent::Ui(UiEvent::Start));
    app.update(AppEvent::Ui(UiEvent::Confirm));
    app.update(AppEvent::Ui(UiEvent::Down));
    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));

    assert!(matches!(
        commands.as_slice(),
        [AppCommand::RequestAgent(prompt), AppCommand::Render]
            if prompt.side == PlayerSide::Local
                && prompt.kind == AgentPromptKind::PlaceShips(PlacementMode::Manual)
    ));
}

#[test]
fn resume_menu_requests_active_save_load() {
    let mut app = new_app();
    app.update(AppEvent::Ui(UiEvent::Start));
    app.update(AppEvent::Ui(UiEvent::Down));
    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));

    assert_eq!(app.state, AppState::MainMenu);
    assert_eq!(commands, vec![AppCommand::LoadActiveSave]);
}

#[test]
fn difficulty_menu_emits_configuration_command() {
    let mut app = new_app();
    app.update(AppEvent::Ui(UiEvent::Start));
    for _ in 0..4 {
        app.update(AppEvent::Ui(UiEvent::Down));
    }
    app.update(AppEvent::Ui(UiEvent::Confirm));
    assert_eq!(app.state, AppState::DifficultyMenu);

    app.update(AppEvent::Ui(UiEvent::Up));
    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));

    assert_eq!(app.state, AppState::MainMenu);
    assert_eq!(app.ai_difficulty, AiDifficulty::Medium);
    assert_eq!(
        commands,
        vec![
            AppCommand::ConfigureDifficulty(AiDifficulty::Medium),
            AppCommand::Render,
        ]
    );
}

#[test]
fn solo_setup_enters_playing_and_orders_commands() {
    let mut app = new_app();
    let commands = setup_solo(&mut app);

    assert_eq!(app.state, AppState::Playing);
    assert!(matches!(
        commands.as_slice(),
        [
            AppCommand::Save(_),
            AppCommand::RequestAgent(prompt),
            AppCommand::Render
        ] if prompt.side == PlayerSide::Local
            && prompt.kind == AgentPromptKind::SelectTarget
    ));
}

#[test]
fn solo_turns_apply_fire_actions_and_save() {
    let mut app = new_app();
    setup_solo(&mut app);

    let commands = app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: AgentAction::Fire((9, 9)),
    });
    assert_eq!(app.match_state.turn_number, 1);
    assert!(!app.match_state.local_turn);
    assert!(matches!(commands.first(), Some(AppCommand::Save(_))));
    assert!(commands.iter().any(|cmd| matches!(
        cmd,
        AppCommand::RequestAgent(prompt)
            if prompt.side == PlayerSide::Opponent
                && prompt.kind == AgentPromptKind::SelectTarget
    )));

    app.update(AppEvent::Agent {
        side: PlayerSide::Opponent,
        action: AgentAction::Fire((9, 9)),
    });
    assert_eq!(app.match_state.turn_number, 2);
    assert!(app.match_state.local_turn);
}

#[test]
fn invalid_agent_action_keeps_state_and_renders() {
    let mut app = new_app();
    setup_solo(&mut app);

    let commands = app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });

    assert_eq!(app.state, AppState::Playing);
    assert_eq!(commands, vec![AppCommand::Render]);
    assert!(app.last_notice.is_some());
}

#[test]
fn loaded_none_clears_save_and_renders() {
    let mut app = new_app();
    app.update(AppEvent::Ui(UiEvent::Start));
    let commands = app.update(AppEvent::Loaded(None));

    assert_eq!(app.state, AppState::MainMenu);
    assert_eq!(app.selected_menu_item, 1);
    assert_eq!(app.last_notice, Some("No saved game."));
    assert_eq!(commands, vec![AppCommand::ClearSave, AppCommand::Render]);
}

#[test]
fn loaded_saved_game_enters_playing_and_requests_next_turn() {
    let mut app = new_app();
    setup_solo(&mut app);
    app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: AgentAction::Fire((9, 9)),
    });
    let saved = app.match_state.saved_game();

    let mut restored = new_app();
    let commands = restored.update(AppEvent::Loaded(Some(saved)));

    assert_eq!(restored.state, AppState::Playing);
    assert_eq!(restored.match_state.turn_number, saved.turn_number);
    assert!(matches!(
        commands.as_slice(),
        [
            AppCommand::Save(SavedGame { .. }),
            AppCommand::RequestAgent(prompt),
            AppCommand::Render
        ] if prompt.side == PlayerSide::Opponent
            && prompt.kind == AgentPromptKind::SelectTarget
    ));
}

#[test]
fn game_over_turn_clears_active_save() {
    let mut app = new_app();
    setup_solo(&mut app);
    let mut last_commands = Vec::new();

    for (index, target) in ship_targets().into_iter().enumerate() {
        last_commands = app.update(AppEvent::Agent {
            side: PlayerSide::Local,
            action: AgentAction::Fire(target),
        });

        if app.state == AppState::GameOver {
            break;
        }

        app.update(AppEvent::Agent {
            side: PlayerSide::Opponent,
            action: AgentAction::Fire(safe_misses()[index]),
        });
    }

    assert_eq!(app.state, AppState::GameOver);
    assert!(matches!(last_commands.first(), Some(AppCommand::ClearSave)));
    assert!(!last_commands
        .iter()
        .any(|command| matches!(command, AppCommand::Save(_))));
}

#[test]
fn remote_pairing_handshake_and_ready_enter_playing() {
    let mut app = new_app();
    app.update(AppEvent::Ui(UiEvent::Start));
    app.update(AppEvent::Ui(UiEvent::Down));
    app.update(AppEvent::Ui(UiEvent::Down));
    let commands = app.update(AppEvent::Ui(UiEvent::Confirm));

    assert_eq!(app.state, AppState::Pairing);
    assert!(matches!(
        commands.first(),
        Some(AppCommand::Send(WireMessage::Handshake {
            version: PROTOCOL_VERSION
        }))
    ));

    let commands = app.update(AppEvent::Transport(WireMessage::HandshakeAck {
        version: PROTOCOL_VERSION,
    }));
    assert!(commands.iter().any(|cmd| matches!(
        cmd,
        AppCommand::RequestAgent(prompt)
            if prompt.side == PlayerSide::Local
                && prompt.kind == AgentPromptKind::PlaceShips(PlacementMode::Random)
    )));

    let commands = app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    assert!(commands.iter().any(|cmd| matches!(
        cmd,
        AppCommand::Send(WireMessage::Ready {
            version: PROTOCOL_VERSION,
            ..
        })
    )));

    let commands = app.update(AppEvent::Transport(WireMessage::Ready {
        version: PROTOCOL_VERSION,
        seq: 1,
    }));
    assert_eq!(app.state, AppState::Playing);
    assert!(matches!(app.match_mode, MatchMode::Remote(_)));
    assert!(commands
        .iter()
        .any(|cmd| matches!(cmd, AppCommand::Save(_))));
}

#[test]
fn remote_duplicate_guess_replays_cached_response() {
    let mut app = new_app();
    app.update(AppEvent::Transport(WireMessage::Handshake {
        version: PROTOCOL_VERSION,
    }));
    app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    app.update(AppEvent::Transport(WireMessage::Ready {
        version: PROTOCOL_VERSION,
        seq: 1,
    }));

    let commands = app.update(AppEvent::Transport(WireMessage::Guess {
        version: PROTOCOL_VERSION,
        seq: 7,
        x: 9,
        y: 9,
    }));
    let first_response = commands
        .iter()
        .find_map(|cmd| match cmd {
            AppCommand::Send(msg @ WireMessage::StatusResp { .. }) => Some(msg.clone()),
            _ => None,
        })
        .expect("first response");

    let commands = app.update(AppEvent::Transport(WireMessage::Guess {
        version: PROTOCOL_VERSION,
        seq: 7,
        x: 9,
        y: 9,
    }));
    assert_eq!(commands.first(), Some(&AppCommand::Send(first_response)));
}

#[test]
fn remote_out_of_order_guess_enters_overlay_and_sends_private_sync() {
    let mut app = new_app();
    app.update(AppEvent::Transport(WireMessage::Handshake {
        version: PROTOCOL_VERSION,
    }));
    app.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    app.update(AppEvent::Transport(WireMessage::Ready {
        version: PROTOCOL_VERSION,
        seq: 1,
    }));
    app.update(AppEvent::Transport(WireMessage::Guess {
        version: PROTOCOL_VERSION,
        seq: 1,
        x: 9,
        y: 9,
    }));

    let commands = app.update(AppEvent::Transport(WireMessage::Guess {
        version: PROTOCOL_VERSION,
        seq: 3,
        x: 8,
        y: 8,
    }));

    assert_eq!(app.state, AppState::ConnectionOverlay);
    assert!(matches!(
        app.match_mode,
        MatchMode::Remote(ref session) if session.status == ConnectionStatus::OutOfSync
    ));
    assert!(commands
        .iter()
        .any(|cmd| matches!(cmd, AppCommand::Send(WireMessage::PrivateSync { .. }))));
}

#[test]
fn private_sync_round_trips_without_board_state() {
    let payload = battleship::domain::RemoteSyncPayload {
        turn_number: 4,
        active_player: RemotePlayer::Local,
        next_seq: 9,
        last_received_seq: Some(8),
        public_shots: GuessBoardState {
            hits: BitBoard::<u128, 10>::new(),
            misses: BitBoard::<u128, 10>::new(),
        },
        enemy_ships_remaining: [true; 5],
        enemy_remaining: 17,
        status: battleship::domain::GameStatus::InProgress,
    };
    let msg = WireMessage::PrivateSync {
        version: PROTOCOL_VERSION,
        seq: 10,
        payload,
    };

    let bytes = bincode::serialize(&msg).unwrap();
    let restored: WireMessage = bincode::deserialize(&bytes).unwrap();
    assert!(matches!(
        restored,
        WireMessage::PrivateSync {
            payload: battleship::domain::RemoteSyncPayload {
                turn_number: 4,
                active_player: RemotePlayer::Local,
                ..
            },
            ..
        }
    ));
}

#[test]
fn two_apps_exchange_remote_guess_over_reducer_events() {
    let mut host = new_app();
    let mut guest = new_app();

    host.match_mode = MatchMode::Remote(battleship::RemoteSession::new(RemoteRole::Host));
    host.match_state = battleship::MatchState::remote_game(RemoteRole::Host);
    host.state = AppState::Pairing;

    let host_commands = host.update(AppEvent::Transport(WireMessage::HandshakeAck {
        version: PROTOCOL_VERSION,
    }));
    assert!(host_commands
        .iter()
        .any(|cmd| matches!(cmd, AppCommand::RequestAgent(_))));

    let guest_commands = guest.update(AppEvent::Transport(WireMessage::Handshake {
        version: PROTOCOL_VERSION,
    }));
    assert!(guest_commands.iter().any(|cmd| matches!(
        cmd,
        AppCommand::Send(WireMessage::HandshakeAck {
            version: PROTOCOL_VERSION
        })
    )));

    host.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    guest.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: placements(),
    });
    host.update(AppEvent::Transport(WireMessage::Ready {
        version: PROTOCOL_VERSION,
        seq: 1,
    }));
    guest.update(AppEvent::Transport(WireMessage::Ready {
        version: PROTOCOL_VERSION,
        seq: 1,
    }));

    let commands = host.update(AppEvent::Agent {
        side: PlayerSide::Local,
        action: AgentAction::Fire((9, 9)),
    });
    let guess = commands
        .into_iter()
        .find_map(|cmd| match cmd {
            AppCommand::Send(msg @ WireMessage::Guess { .. }) => Some(msg),
            _ => None,
        })
        .expect("guess message");

    let commands = guest.update(AppEvent::Transport(guess));
    let response = commands
        .into_iter()
        .find_map(|cmd| match cmd {
            AppCommand::Send(msg @ WireMessage::StatusResp { .. }) => Some(msg),
            _ => None,
        })
        .expect("status response");

    host.update(AppEvent::Transport(response));
    assert_eq!(host.match_state.turn_number, 1);
    assert!(!host.match_state.local_turn);
}
