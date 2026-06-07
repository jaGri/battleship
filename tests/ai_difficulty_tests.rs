use battleship::{
    calc_pdf, sample_pdf, AiAgent, AiDifficulty, BattleshipApp, BitBoard, Board, GuessBoard,
};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::collections::HashMap;

type BB = BitBoard<u128, 10>;

#[test]
fn difficulty_constructors_and_builders_work() {
    let _easy = AiAgent::new(AiDifficulty::Easy);
    let _medium = AiAgent::new(AiDifficulty::Medium);
    let _hard = AiAgent::new(AiDifficulty::Hard);
    let _expert = AiAgent::new(AiDifficulty::Expert);
    let _custom = AiAgent::with_params(0.5, 10.0)
        .with_temperature(1.0)
        .with_hit_weight(15.0);
}

#[test]
fn temperature_affects_sampling() {
    let mut rng = SmallRng::seed_from_u64(42);
    let mut pdf = [[0.01f64; 10]; 10];
    pdf[5][5] = 0.9;

    let mut low_temp_picks: HashMap<(usize, usize), usize> = HashMap::new();
    for _ in 0..100 {
        let pick = sample_pdf(&pdf, 0.1, &mut rng);
        *low_temp_picks.entry(pick).or_insert(0) += 1;
    }

    let mut high_temp_picks: HashMap<(usize, usize), usize> = HashMap::new();
    for _ in 0..100 {
        let pick = sample_pdf(&pdf, 2.0, &mut rng);
        *high_temp_picks.entry(pick).or_insert(0) += 1;
    }

    let low_temp_count = low_temp_picks.get(&(5, 5)).copied().unwrap_or(0);
    let high_temp_count = high_temp_picks.get(&(5, 5)).copied().unwrap_or(0);

    assert!(low_temp_count > high_temp_count);
    assert!(low_temp_count >= 70);
}

#[test]
fn ai_agents_place_and_select_targets() {
    for difficulty in [
        AiDifficulty::Easy,
        AiDifficulty::Medium,
        AiDifficulty::Hard,
        AiDifficulty::Expert,
    ] {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut ai = AiAgent::new(difficulty);
        let mut board = Board::new();
        ai.place_ships(&mut rng, &mut board).unwrap();

        let guess_board = GuessBoard {
            hits: BB::new(),
            misses: BB::new(),
            active_hits: BB::new(),
        };
        let target = ai.select_target(&mut rng, &guess_board, &[5, 4, 3, 3, 2]);
        assert!(target.0 < 10 && target.1 < 10);
    }
}

#[test]
fn sunk_hits_do_not_boost_ai_targeting_pdf() {
    let mut hits = BB::new();
    hits.set(4, 0).unwrap();
    hits.set(4, 1).unwrap();
    let active_hits = BB::new();
    let misses = BB::new();
    let sunk = hits;
    let remaining = [5, 4, 3, 3, 0];

    let pdf_with_sunk_filter = calc_pdf(&active_hits, &misses, &sunk, &remaining, 13.0);
    let pdf_without_sunk_filter = calc_pdf(&hits, &misses, &BB::new(), &remaining, 13.0);

    assert!(pdf_without_sunk_filter[4][2] > pdf_with_sunk_filter[4][2]);
}

#[test]
fn expert_beats_easy_in_small_sample() {
    let mut expert_wins = 0;
    for game_num in 0..10 {
        if play_ai_vs_ai(
            AiDifficulty::Expert,
            AiDifficulty::Easy,
            (game_num * 2) as u64,
            (game_num * 2 + 1) as u64,
        ) {
            expert_wins += 1;
        }
    }
    assert!(expert_wins >= 7, "expert wins: {}", expert_wins);
}

fn play_ai_vs_ai(higher: AiDifficulty, lower: AiDifficulty, seed1: u64, seed2: u64) -> bool {
    let mut rng1 = SmallRng::seed_from_u64(seed1);
    let mut rng2 = SmallRng::seed_from_u64(seed2);
    let mut app = BattleshipApp::new_local_ai(AiAgent::new(higher), AiAgent::new(lower));
    app.place_ships(&mut rng1, &mut rng2).unwrap();
    for _ in 0..300 {
        if app.state == battleship::AppState::GameOver {
            return app.match_state.local_engine.status() == battleship::GameStatus::Won;
        }
        app.play_next_turn(&mut rng1, &mut rng2).unwrap();
    }
    panic!("game did not complete");
}
