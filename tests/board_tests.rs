use battleship::{BoardError, BoardState, GuessResult, Orientation, SHIPS};
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
    board.place_random(&mut rng).unwrap();
    let total: usize = SHIPS.iter().map(|s| s.length()).sum();
    assert_eq!(board.ship_map().count_ones(), total);
}
