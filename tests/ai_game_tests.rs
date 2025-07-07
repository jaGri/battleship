use battleship::{AiPlayer, GameEngine, GameStatus, Player};
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[test]
fn test_ai_vs_ai_game() {
    let mut rng = SmallRng::seed_from_u64(123);
    let mut p1 = AiPlayer::new();
    let mut p2 = AiPlayer::new();
    let mut e1 = GameEngine::new();
    let mut e2 = GameEngine::new();
    p1.place_ships(&mut rng, e1.board_mut()).unwrap();
    p2.place_ships(&mut rng, e2.board_mut()).unwrap();

    let mut turns = 0;
    loop {
        turns += 1;
        // p1 turn
        let guess = p1.select_target(
            &mut rng,
            &e1.guess_hits(),
            &e1.guess_misses(),
            &e1.enemy_ship_lengths_remaining(),
        );
        let res = e2.opponent_guess(guess.0, guess.1).unwrap();
        e1.record_guess(guess.0, guess.1, res).unwrap();
        p1.handle_guess_result(guess, res);
        if e2.status() == GameStatus::Lost {
            break;
        }
        // p2 turn
        let guess = p2.select_target(
            &mut rng,
            &e2.guess_hits(),
            &e2.guess_misses(),
            &e2.enemy_ship_lengths_remaining(),
        );
        let res = e1.opponent_guess(guess.0, guess.1).unwrap();
        e2.record_guess(guess.0, guess.1, res).unwrap();
        p2.handle_guess_result(guess, res);
        if e1.status() == GameStatus::Lost {
            break;
        }
        if turns > 200 {
            panic!("game took too many turns");
        }
    }
    assert!(matches!(e1.status(), GameStatus::Won | GameStatus::Lost));
    assert!(matches!(e2.status(), GameStatus::Won | GameStatus::Lost));
}
