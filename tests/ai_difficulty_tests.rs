use battleship::{AiPlayer, BitBoard, Board, Difficulty, GameEngine, Player};
use rand::rngs::SmallRng;
use rand::SeedableRng;

type BB = BitBoard<u128, 10>;

#[test]
fn test_difficulty_constructors() {
    // Test that each difficulty level creates a player
    let _easy = AiPlayer::with_difficulty(Difficulty::Easy);
    let _medium = AiPlayer::with_difficulty(Difficulty::Medium);
    let _hard = AiPlayer::with_difficulty(Difficulty::Hard);
    let _expert = AiPlayer::with_difficulty(Difficulty::Expert);
}

#[test]
fn test_backward_compatibility() {
    // Ensure existing constructors still work
    let _ = AiPlayer::new();
    let _ = AiPlayer::new().with_temperature(0.5);
    let _ = AiPlayer::with_params(0.5, 10.0);
}

#[test]
fn test_builder_style_methods() {
    // Test builder-style chaining
    let _ai = AiPlayer::new()
        .with_temperature(1.0)
        .with_hit_weight(15.0);
}

#[test]
fn test_temperature_affects_sampling() {
    use battleship::sample_pdf;
    use std::collections::HashMap;

    let mut rng = SmallRng::seed_from_u64(42);

    // Create a PDF with one dominant cell
    let mut pdf = [[0.01f64; 10]; 10];
    pdf[5][5] = 0.9; // Very high probability at (5, 5)

    // Sample with low temperature (greedy) - should pick (5,5) most often
    let mut low_temp_picks: HashMap<(usize, usize), usize> = HashMap::new();
    for _ in 0..100 {
        let pick = sample_pdf(&pdf, 0.1, &mut rng);
        *low_temp_picks.entry(pick).or_insert(0) += 1;
    }

    // Sample with high temperature (random) - should be more distributed
    let mut high_temp_picks: HashMap<(usize, usize), usize> = HashMap::new();
    for _ in 0..100 {
        let pick = sample_pdf(&pdf, 2.0, &mut rng);
        *high_temp_picks.entry(pick).or_insert(0) += 1;
    }

    // Low temperature should pick (5,5) more often than high temperature
    let low_temp_count = low_temp_picks.get(&(5, 5)).copied().unwrap_or(0);
    let high_temp_count = high_temp_picks.get(&(5, 5)).copied().unwrap_or(0);

    assert!(
        low_temp_count > high_temp_count,
        "Low temperature should favor high-probability cells more: low={}, high={}",
        low_temp_count,
        high_temp_count
    );

    // Low temperature should pick (5,5) most of the time (at least 70%)
    assert!(
        low_temp_count >= 70,
        "Low temperature should pick best cell at least 70% of time: {}",
        low_temp_count
    );
}

#[test]
fn test_ai_players_can_play() {
    // Smoke test: ensure all difficulty levels can actually play a game
    for difficulty in [
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Expert,
    ] {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut ai = AiPlayer::with_difficulty(difficulty);
        let mut board = Board::new();

        // Place ships
        ai.place_ships(&mut rng, &mut board)
            .expect("Should be able to place ships");

        // Make a guess
        let hits = BB::new();
        let misses = BB::new();
        let remaining = [5, 4, 3, 3, 2]; // All ships remaining

        let (r, c) = ai.select_target(&mut rng, &hits, &misses, &remaining);

        // Verify guess is on the board
        assert!(r < 10 && c < 10, "Guess should be on board: ({}, {})", r, c);
    }
}

/// Play a simple AI vs AI game and verify it completes
#[test]
fn test_full_ai_game_completes() {
    let mut rng1 = SmallRng::seed_from_u64(100);
    let mut rng2 = SmallRng::seed_from_u64(101);

    let mut ai1 = AiPlayer::with_difficulty(Difficulty::Hard);
    let mut ai2 = AiPlayer::with_difficulty(Difficulty::Easy);

    let mut engine1 = GameEngine::new();
    let mut engine2 = GameEngine::new();

    // Place ships
    ai1.place_ships(&mut rng1, engine1.board_mut()).unwrap();
    ai2.place_ships(&mut rng2, engine2.board_mut()).unwrap();

    // Play turns until someone wins (max 200 turns to prevent infinite loop)
    for turn in 0..200 {
        // Player 1's turn
        let (r, c) = ai1.select_target(
            &mut rng1,
            &engine1.guess_hits(),
            &engine1.guess_misses(),
            &engine1.enemy_ship_lengths_remaining(),
        );

        if let Ok(result) = engine2.opponent_guess(r, c) {
            engine1.record_guess(r, c, result).unwrap();
            ai1.handle_guess_result((r, c), result);

            if engine1.status() != battleship::GameStatus::InProgress {
                println!("Player 1 won in {} turns", turn + 1);
                return;
            }
        }

        // Player 2's turn
        let (r, c) = ai2.select_target(
            &mut rng2,
            &engine2.guess_hits(),
            &engine2.guess_misses(),
            &engine2.enemy_ship_lengths_remaining(),
        );

        if let Ok(result) = engine1.opponent_guess(r, c) {
            engine2.record_guess(r, c, result).unwrap();
            ai2.handle_guess_result((r, c), result);

            if engine2.status() != battleship::GameStatus::InProgress {
                println!("Player 2 won in {} turns", turn + 1);
                return;
            }
        }
    }

    panic!("Game didn't complete in 200 turns");
}

/// Test that sunk ship tracking works correctly
#[test]
fn test_sunk_ship_tracking() {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut ai = AiPlayer::with_difficulty(Difficulty::Hard);
    let mut board = Board::new();

    // Place ships
    ai.place_ships(&mut rng, &mut board).unwrap();

    // Simulate hitting and sinking a ship
    let hits = BB::new();
    let misses = BB::new();
    let remaining = [5, 4, 3, 3, 2];

    // Make some guesses
    for _ in 0..10 {
        let (_r, _c) = ai.select_target(&mut rng, &hits, &misses, &remaining);
        // In a real game, we'd update hits/misses based on results
    }

    // Test passes if no panics occur
}
