use battleship::{AiAgent, AiDifficulty, AppState, BattleshipApp, GameStatus};
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[test]
fn ai_agents_complete_a_local_app_game() {
    let mut rng1 = SmallRng::seed_from_u64(123);
    let mut rng2 = SmallRng::seed_from_u64(456);
    let mut app = BattleshipApp::new_local_ai(
        AiAgent::new(AiDifficulty::Hard),
        AiAgent::new(AiDifficulty::Hard),
    );

    app.place_ships(&mut rng1, &mut rng2).unwrap();
    assert_eq!(app.state, AppState::Playing);

    for _ in 0..300 {
        if app.state == AppState::GameOver {
            break;
        }
        app.play_next_turn(&mut rng1, &mut rng2).unwrap();
    }

    assert_eq!(app.state, AppState::GameOver);
    assert!(matches!(
        app.match_state.local_engine.status(),
        GameStatus::Won | GameStatus::Lost
    ));
}
