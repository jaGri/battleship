use battleship::{Board, BoardError, BoardState, BOARD_SIZE, NUM_SHIPS};
use proptest::prelude::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};

fn random_board(seed: u64) -> Board {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut board = Board::new();
    for i in 0..NUM_SHIPS as usize {
        let (r, c, orient) = board.random_placement(&mut rng, i).unwrap();
        board.place(i, r, c, orient).unwrap();
    }
    let guesses = rng.random_range(0..BOARD_SIZE as usize);
    for _ in 0..guesses {
        let r = rng.random_range(0..BOARD_SIZE as usize);
        let c = rng.random_range(0..BOARD_SIZE as usize);
        let _ = board.guess(r, c);
    }
    board
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn board_state_roundtrip(seed in any::<u64>()) {
        let board = random_board(seed);
        let state1 = BoardState::from(&board);
        let board2: Board = state1.into();
        let state2 = BoardState::from(&board2);
        prop_assert_eq!(state1, state2);
    }

    #[test]
    fn guess_idempotent(seed in any::<u64>(), row in 0..BOARD_SIZE as usize, col in 0..BOARD_SIZE as usize) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut board = Board::new();
        for i in 0..NUM_SHIPS as usize {
            let (r, c, orient) = board.random_placement(&mut rng, i).unwrap();
            board.place(i, r, c, orient).unwrap();
        }
        let state_before = BoardState::from(&board);
        board.guess(row, col).unwrap();
        let state_after = BoardState::from(&board);
        let err = board.guess(row, col).unwrap_err();
        prop_assert_eq!(err, BoardError::AlreadyGuessed);
        prop_assert_eq!(BoardState::from(&board), state_after);
        prop_assert_ne!(state_before, state_after);
    }
}

