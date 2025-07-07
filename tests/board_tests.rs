use battleship::{Board, BoardError, GuessResult, Orientation, BOARD_SIZE, NUM_SHIPS, SHIPS};
use battleship::{BoardState, Ship};
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[test]
fn test_manual_place_and_guess_sink() {
    let mut board = Board::new();
    board.place(0, 0, 0, Orientation::Horizontal).unwrap();

    for c in 0..SHIPS[0].length() - 1 {
        assert_eq!(board.guess(0, c).unwrap(), GuessResult::Hit);
    }
    // final hit should sink
    assert_eq!(
        board.guess(0, SHIPS[0].length() - 1).unwrap(),
        GuessResult::Sink("Carrier")
    );
    assert!(board.ship_states()[0].sunk);

    // repeated guess triggers error
    assert_eq!(
        board.guess(0, SHIPS[0].length() - 1).unwrap_err(),
        BoardError::AlreadyGuessed
    );
}

#[test]
fn test_place_random_no_overlap() {
    let mut board = Board::new();
    let mut rng = SmallRng::seed_from_u64(42);
    let ship_index = 0; // Carrier
    let (r, c, orient) = board.random_placement(&mut rng, ship_index).unwrap();
    board.place(ship_index, r, c, orient).unwrap();
    let expected = SHIPS[ship_index].length();
    assert_eq!(board.ship_map().count_ones(), expected);
}

#[test]
fn test_place_random_all_ships_no_overlap() {
    let mut board = Board::new();
    let mut rng = SmallRng::seed_from_u64(42);

    let mut expected_bits = 0;
    for i in 0..NUM_SHIPS as usize {
        let (r, c, orient) = board.random_placement(&mut rng, i).unwrap();
        board.place(i, r, c, orient).unwrap();
        expected_bits += SHIPS[i].length();
    }

    assert_eq!(
        board.ship_map().count_ones(),
        expected_bits,
        "all ships should be placed without overlap"
    );
}

#[test]
fn test_board_state_roundtrip() {
    let mut board = Board::new();
    board.place(1, 2, 2, Orientation::Vertical).unwrap();
    board.guess(2, 2).unwrap();

    let state = BoardState::from(&board);
    let mut board2: Board = state.into();

    assert_eq!(board2.guess(2, 2).unwrap_err(), BoardError::AlreadyGuessed);
    assert_eq!(
        board2.ship_states()[1].position,
        Some((2, 2, Orientation::Vertical))
    );
}

#[test]
fn test_ship_state_conversion() {
    let mut board = Board::new();
    board.place(2, 4, 1, Orientation::Horizontal).unwrap();
    let states = board.ship_states();
    let def = SHIPS[2];
    let ship = Ship::<u128, { BOARD_SIZE as usize }>::from_state(&states[2], def)
        .unwrap()
        .unwrap();
    assert_eq!(ship.origin(), (4, 1));
    assert_eq!(ship.orientation(), Orientation::Horizontal);
}
