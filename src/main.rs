use battleship::{AiPlayer, CliPlayer, GameEngine, GameStatus, Player, print_player_view};
use rand::thread_rng;

fn main() {
    let mut rng = thread_rng();
    let mut cli = CliPlayer::new();
    let mut ai = AiPlayer::new();
    let mut my_engine = GameEngine::new();
    let mut ai_engine = GameEngine::new();

    cli.place_ships(&mut rng, my_engine.board_mut())
        .expect("placement");
    ai.place_ships(&mut rng, ai_engine.board_mut())
        .expect("placement");

    loop {
        // show current boards before taking a turn
        print_player_view(&my_engine);

        // player turn
        let guess = cli.select_target(
            &mut rng,
            &my_engine.guess_hits(),
            &my_engine.guess_misses(),
            &my_engine.enemy_ship_lengths_remaining(),
        );
        let res = ai_engine.opponent_guess(guess.0, guess.1).expect("guess");
        my_engine
            .record_guess(guess.0, guess.1, res)
            .expect("record");
        cli.handle_guess_result(guess, res);
        print_player_view(&my_engine);
        if ai_engine.status() == GameStatus::Lost {
            println!("You won!");
            break;
        }

        // ai turn
        let guess = ai.select_target(
            &mut rng,
            &ai_engine.guess_hits(),
            &ai_engine.guess_misses(),
            &ai_engine.enemy_ship_lengths_remaining(),
        );
        let res = my_engine.opponent_guess(guess.0, guess.1).expect("guess");
        ai_engine
            .record_guess(guess.0, guess.1, res)
            .expect("record");
        cli.handle_opponent_guess(guess, res);
        print_player_view(&my_engine);
        if my_engine.status() == GameStatus::Lost {
            println!("You lost!");
            break;
        }
    }
}
