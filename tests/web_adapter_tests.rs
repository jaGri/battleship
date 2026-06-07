#![cfg(feature = "web")]

use battleship::render::{ConnectionView, GameView, MenuView, MessageView};
use battleship::{
    AppEvent, BattleshipApp, Board, GameStatus, GuessBoard, InputSource, Orientation, Renderer,
    ScreenView, ScriptedAgent, UiEvent, WebBoardCell, WebConnectionView, WebGameEvent,
    WebGameStatus, WebGuessCell, WebGuessResult, WebInput, WebInputError, WebInputEvent,
    WebMenuView, WebMessageView, WebRenderer, WebScreenView, BOARD_SIZE,
};

fn board_with_visible_states() -> Board {
    let mut board = Board::new();
    board.place(0, 4, 0, Orientation::Horizontal).unwrap();
    board.place(2, 2, 0, Orientation::Horizontal).unwrap();
    board.place(4, 0, 0, Orientation::Horizontal).unwrap();

    assert_eq!(board.guess(0, 0).unwrap(), battleship::GuessResult::Hit);
    assert!(matches!(
        board.guess(0, 1).unwrap(),
        battleship::GuessResult::Sink("Destroyer")
    ));
    assert_eq!(board.guess(2, 0).unwrap(), battleship::GuessResult::Hit);
    assert_eq!(board.guess(9, 9).unwrap(), battleship::GuessResult::Miss);

    board
}

fn guess_board_with_visible_states() -> GuessBoard {
    let mut board = GuessBoard::new();
    board.hits.set(1, 1).unwrap();
    board.active_hits.set(1, 1).unwrap();
    board.hits.set(3, 3).unwrap();
    board.misses.set(2, 2).unwrap();
    board
}

#[test]
fn web_input_events_map_to_ui_events() {
    let mappings = [
        (WebInputEvent::Up, UiEvent::Up),
        (WebInputEvent::Down, UiEvent::Down),
        (WebInputEvent::Left, UiEvent::Left),
        (WebInputEvent::Right, UiEvent::Right),
        (WebInputEvent::Confirm, UiEvent::Confirm),
        (WebInputEvent::Back, UiEvent::Back),
        (WebInputEvent::Start, UiEvent::Start),
        (WebInputEvent::ConnectionMenu, UiEvent::ConnectionMenu),
        (WebInputEvent::Tick, UiEvent::Tick),
        (
            WebInputEvent::Target { row: 3, col: 4 },
            UiEvent::Target((3, 4)),
        ),
    ];

    let mut input = WebInput::new();
    for (web_event, _) in mappings {
        input.push_event(web_event).unwrap();
    }

    for (_, expected) in mappings {
        assert_eq!(input.poll_input().unwrap(), Some(expected));
    }
    assert_eq!(input.poll_input().unwrap(), None);
}

#[test]
fn web_input_rejects_out_of_bounds_targets_without_enqueueing() {
    let mut input = WebInput::new();

    let err = input
        .push_event(WebInputEvent::Target {
            row: BOARD_SIZE as usize,
            col: 0,
        })
        .unwrap_err();

    assert_eq!(
        err,
        WebInputError::TargetOutOfBounds {
            row: BOARD_SIZE as usize,
            col: 0
        }
    );
    assert!(input.is_empty());
    assert_eq!(input.poll_input().unwrap(), None);
}

#[test]
fn web_renderer_builds_owned_game_view_from_real_board_state() {
    let board = board_with_visible_states();
    let guess_board = guess_board_with_visible_states();
    let view = ScreenView::Game(GameView {
        my_board: &board,
        guess_board,
        my_turn: true,
        turn_number: 7,
        status: GameStatus::InProgress,
        last_event: Some(battleship::render::GameEventView::Guess {
            coord: (2, 0),
            result: battleship::GuessResult::Hit,
            by_local_player: false,
        }),
    });

    let mut renderer = WebRenderer::new();
    renderer.render(&view).unwrap();

    let WebScreenView::Game(game) = renderer.latest().unwrap() else {
        panic!("expected game view");
    };
    assert_eq!(game.my_board.size, 10);
    assert_eq!(game.my_board.cells.len(), 10);
    assert!(game.my_board.cells.iter().all(|row| row.len() == 10));
    assert_eq!(game.my_board.cells[0][0], WebBoardCell::Sunk);
    assert_eq!(game.my_board.cells[0][1], WebBoardCell::Sunk);
    assert_eq!(game.my_board.cells[2][0], WebBoardCell::Hit);
    assert_eq!(game.my_board.cells[4][0], WebBoardCell::Ship);
    assert_eq!(game.my_board.cells[9][9], WebBoardCell::Miss);
    assert_eq!(game.my_board.cells[8][8], WebBoardCell::Water);
    assert_eq!(game.my_board.ships.len(), 5);
    assert!(game
        .my_board
        .ships
        .iter()
        .any(|ship| ship.name == "Destroyer" && ship.length == 2 && ship.sunk));

    assert_eq!(game.guess_board.size, 10);
    assert_eq!(game.guess_board.cells[1][1], WebGuessCell::ActiveHit);
    assert_eq!(game.guess_board.cells[3][3], WebGuessCell::Hit);
    assert_eq!(game.guess_board.cells[2][2], WebGuessCell::Miss);
    assert_eq!(game.guess_board.cells[0][0], WebGuessCell::Unknown);
    assert_eq!(game.turn_number, 7);
    assert_eq!(game.status, WebGameStatus::InProgress);
    assert_eq!(
        game.last_event,
        Some(WebGameEvent::Guess {
            row: 2,
            col: 0,
            result: WebGuessResult::Hit,
            by_local_player: false,
        })
    );
}

#[test]
fn web_renderer_preserves_owned_non_game_views() {
    let menu_items = ["Solo Game", "Remote Game"];
    assert_eq!(
        WebScreenView::from(&ScreenView::Menu(MenuView {
            title: "Battleship",
            items: &menu_items,
            selected: 1,
        })),
        WebScreenView::Menu(WebMenuView {
            title: "Battleship".to_string(),
            items: vec!["Solo Game".to_string(), "Remote Game".to_string()],
            selected: 1,
        })
    );

    assert_eq!(
        WebScreenView::from(&ScreenView::Message(MessageView {
            title: "Notice",
            body: "Ready",
        })),
        WebScreenView::Message(WebMessageView {
            title: "Notice".to_string(),
            body: "Ready".to_string(),
        })
    );

    assert_eq!(
        WebScreenView::from(&ScreenView::Pairing(ConnectionView {
            code: Some("ABCD"),
            connected: true,
            status: "connected",
        })),
        WebScreenView::Pairing(WebConnectionView {
            code: Some("ABCD".to_string()),
            connected: true,
            status: "connected".to_string(),
        })
    );

    assert_eq!(
        WebScreenView::from(&ScreenView::ConnectionOverlay(ConnectionView {
            code: None,
            connected: false,
            status: "reconnecting",
        })),
        WebScreenView::ConnectionOverlay(WebConnectionView {
            code: None,
            connected: false,
            status: "reconnecting".to_string(),
        })
    );
}

#[test]
fn app_flow_consumes_web_input_and_renders_web_view() {
    let mut app = BattleshipApp::new_local_ai(ScriptedAgent::default(), ScriptedAgent::default());
    let mut input = WebInput::new();
    let mut renderer = WebRenderer::new();

    input.push_event(WebInputEvent::Start).unwrap();
    let event = input.poll_input().unwrap().expect("queued start event");
    app.update(AppEvent::Ui(event));
    renderer.render(&app.view()).unwrap();

    let WebScreenView::Menu(menu) = renderer.latest().unwrap() else {
        panic!("expected menu view");
    };
    assert_eq!(menu.title, "Battleship");
    assert_eq!(menu.selected, 0);
}
