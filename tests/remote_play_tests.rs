#![cfg(all(feature = "std", feature = "in-memory"))]

use battleship::{
    AgentAction, AgentPromptKind, AppCommand, AppEvent, AppState, BattleshipApp, InMemoryTransport,
    Orientation, PlayerSide, ScriptedAgent, ShipPlacement, TransportCommandRunner, UiEvent,
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

fn start_remote_host(app: &mut BattleshipApp<ScriptedAgent, ScriptedAgent>) -> Vec<AppCommand> {
    app.update(AppEvent::Ui(UiEvent::Start));
    app.update(AppEvent::Ui(UiEvent::Down));
    app.update(AppEvent::Ui(UiEvent::Down));
    app.update(AppEvent::Ui(UiEvent::Confirm))
}

fn answer_prompts(
    app: &mut BattleshipApp<ScriptedAgent, ScriptedAgent>,
    commands: &mut Vec<AppCommand>,
    mut target: Option<(usize, usize)>,
) {
    let mut next = Vec::new();
    for command in commands.drain(..) {
        match command {
            AppCommand::RequestAgent(prompt) => match prompt.kind {
                AgentPromptKind::PlaceShips(_) => {
                    next.extend(app.update(AppEvent::Agent {
                        side: prompt.side,
                        action: placements(),
                    }));
                }
                AgentPromptKind::SelectTarget
                    if prompt.side == PlayerSide::Local && target.is_some() =>
                {
                    next.extend(app.update(AppEvent::Agent {
                        side: prompt.side,
                        action: AgentAction::Fire(target.take().unwrap()),
                    }));
                }
                AgentPromptKind::Observe(_) => {}
                _ => next.push(AppCommand::RequestAgent(prompt)),
            },
            other => next.push(other),
        }
    }
    *commands = next;
}

fn pump_pair(
    host: &mut BattleshipApp<ScriptedAgent, ScriptedAgent>,
    host_runner: &mut TransportCommandRunner<InMemoryTransport>,
    host_commands: &mut Vec<AppCommand>,
    guest: &mut BattleshipApp<ScriptedAgent, ScriptedAgent>,
    guest_runner: &mut TransportCommandRunner<InMemoryTransport>,
    guest_commands: &mut Vec<AppCommand>,
) {
    host_runner.pump(host, host_commands).unwrap();
    guest_runner.pump(guest, guest_commands).unwrap();
}

#[test]
fn in_memory_runner_bridges_remote_setup_and_guess_exchange() {
    let (host_transport, guest_transport) = InMemoryTransport::pair();
    let mut host_runner = TransportCommandRunner::new(host_transport);
    let mut guest_runner = TransportCommandRunner::new(guest_transport);
    let mut host = new_app();
    let mut guest = new_app();

    let mut host_commands = start_remote_host(&mut host);
    let mut guest_commands = Vec::new();

    for _ in 0..8 {
        pump_pair(
            &mut host,
            &mut host_runner,
            &mut host_commands,
            &mut guest,
            &mut guest_runner,
            &mut guest_commands,
        );
        answer_prompts(&mut host, &mut host_commands, None);
        answer_prompts(&mut guest, &mut guest_commands, None);
    }

    assert_eq!(host.state, AppState::Playing);
    assert_eq!(guest.state, AppState::Playing);
    assert!(host.match_state.local_turn);
    assert!(!guest.match_state.local_turn);

    answer_prompts(&mut host, &mut host_commands, Some((9, 9)));
    for _ in 0..4 {
        pump_pair(
            &mut host,
            &mut host_runner,
            &mut host_commands,
            &mut guest,
            &mut guest_runner,
            &mut guest_commands,
        );
    }

    assert_eq!(host.match_state.turn_number, 1);
    assert_eq!(guest.match_state.turn_number, 1);
    assert!(!host.match_state.local_turn);
    assert!(guest.match_state.local_turn);
}
