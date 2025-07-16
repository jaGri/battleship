#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
use battleship::{
    transport::in_memory::InMemoryTransport,
    AiPlayer,
    CliPlayer,
    GameEngine,
    GameStatus,
    Player,
    PlayerNode,
    print_player_view,
    print_probability_board,
    calc_pdf,
};

#[cfg(feature = "std")]
use rand::rngs::SmallRng;
#[cfg(feature = "std")]
use rand::SeedableRng;

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut seed = rand::rng();
    let mut rng_cli = SmallRng::from_rng(&mut seed);
    let mut rng_ai = SmallRng::from_rng(&mut seed);

    let mut cli = CliPlayer::new();
    let mut ai = AiPlayer::new();
    let mut cli_engine = GameEngine::new();
    let mut ai_engine = GameEngine::new();

    cli
        .place_ships(&mut rng_cli, cli_engine.board_mut())
        .map_err(|e| anyhow::anyhow!(e))?;
    ai
        .place_ships(&mut rng_ai, ai_engine.board_mut())
        .map_err(|e| anyhow::anyhow!(e))?;

    let (t_cli, t_ai) = InMemoryTransport::pair();

    let ai_future = async move {
        let mut node = PlayerNode::new(Box::new(ai), ai_engine, Box::new(t_ai));
        node.run(&mut rng_ai, false).await
    };

    let cli_future = run_cli(cli, cli_engine, Box::new(t_cli), rng_cli);

    tokio::try_join!(cli_future, ai_future)?;
    Ok(())
}

#[cfg(feature = "std")]
async fn run_cli(
    mut player: CliPlayer,
    mut engine: GameEngine,
    mut transport: Box<dyn battleship::transport::Transport>,
    mut rng: SmallRng,
) -> anyhow::Result<()> {
    let mut my_turn = true;
    loop {
        if my_turn {
            print_player_view(&engine);
            let pdf = calc_pdf(
                &engine.guess_hits(),
                &engine.guess_misses(),
                &engine.enemy_ship_lengths_remaining(),
            );
            print_probability_board(&pdf);

            let (r, c) = player.select_target(
                &mut rng,
                &engine.guess_hits(),
                &engine.guess_misses(),
                &engine.enemy_ship_lengths_remaining(),
            );
            transport
                .send(battleship::Message::Guess { x: r as u8, y: c as u8 })
                .await?;
            let reply = transport.recv().await?;
            let res_domain = match reply {
                battleship::Message::StatusResp(res) => res,
                _ => return Err(anyhow::anyhow!("unexpected reply")),
            };
            let res_common = match res_domain {
                battleship::domain::GuessResult::Hit => battleship::GuessResult::Hit,
                battleship::domain::GuessResult::Miss => battleship::GuessResult::Miss,
                battleship::domain::GuessResult::Sink => battleship::GuessResult::Hit,
            };
            engine.record_guess(r, c, res_common).map_err(|e| anyhow::anyhow!(e))?;
            player.handle_guess_result((r, c), res_common);
            my_turn = false;
        } else {
            let msg = transport.recv().await?;
            if let battleship::Message::Guess { x, y } = msg {
                let res_common = engine
                    .opponent_guess(x as usize, y as usize)
                    .map_err(|e| anyhow::anyhow!(e))?;
                player.handle_opponent_guess((x as usize, y as usize), res_common);
                let res_domain = battleship::domain::GuessResult::from(res_common);
                transport
                    .send(battleship::Message::StatusResp(res_domain))
                    .await?;
            } else {
                continue;
            }
            my_turn = true;
        }

        if !matches!(engine.status(), GameStatus::InProgress) {
            break;
        }
    }
    print_player_view(&engine);
    match engine.status() {
        GameStatus::Won => println!("You won!"),
        GameStatus::Lost => println!("You lost!"),
        _ => {}
    }
    Ok(())
}
