#![cfg(feature = "std")]

use battleship::{
    AgentAction, AgentRequest, Board, HumanAgent, Orientation, PlayerAgent, ShipPlacement, UiEvent,
    NUM_SHIPS,
};
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn placements() -> Vec<ShipPlacement> {
    vec![
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
    ]
}

#[test]
fn human_agent_returns_manual_ship_placements_from_ui_events() {
    let expected = placements();
    let mut agent = HumanAgent::default();
    let mut rng = SmallRng::seed_from_u64(7);
    let board = Board::new();

    agent.on_ui_event(UiEvent::ClearPlacements);
    for placement in &expected {
        agent.on_ui_event(UiEvent::PlaceShip {
            ship_index: placement.ship_index,
            row: placement.row,
            col: placement.col,
            orientation: placement.orientation,
        });
    }

    let action = agent
        .handle_request(AgentRequest::PlaceShips { board: &board }, &mut rng)
        .unwrap();

    assert_eq!(action, AgentAction::PlaceShips(expected));
}

#[test]
fn human_agent_random_placement_event_returns_legal_complete_fleet() {
    let mut agent = HumanAgent::default();
    let mut rng = SmallRng::seed_from_u64(11);
    let board = Board::new();

    agent.on_ui_event(UiEvent::RandomPlacement);
    let action = agent
        .handle_request(AgentRequest::PlaceShips { board: &board }, &mut rng)
        .unwrap();

    let AgentAction::PlaceShips(placements) = action else {
        panic!("expected placements");
    };
    assert_eq!(placements.len(), NUM_SHIPS);

    let mut placed_board = Board::new();
    for placement in placements {
        placed_board
            .place(
                placement.ship_index,
                placement.row,
                placement.col,
                placement.orientation,
            )
            .unwrap();
    }
}
