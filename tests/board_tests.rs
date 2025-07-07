use battleship::{BoardError, BoardState, GuessResult, Orientation, NUM_SHIPS, SHIPS};
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[test]
fn test_manual_place_and_guess_sink() {
    let mut board = BoardState::new();
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
    let mut board = BoardState::new();
    let mut rng = SmallRng::seed_from_u64(42);
    let ship_index = 0; // Carrier
    let (r, c, orient) = board.random_placement(&mut rng, ship_index).unwrap();
    board.place(ship_index, r, c, orient).unwrap();
    let expected = SHIPS[ship_index].length();
    assert_eq!(board.ship_map().count_ones(), expected);
}

#[test]
fn test_place_random_all_ships_no_overlap() {
    let mut board = BoardState::new();
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