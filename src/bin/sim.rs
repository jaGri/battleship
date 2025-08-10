use battleship::{AiPlayer, GameEngine, GameStatus, Player, PlayerNode, transport::in_memory::InMemoryTransport};
use rand::{rngs::SmallRng, SeedableRng};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <seed1> <seed2>", args[0]);
        std::process::exit(1);
    }
    let seed1: u64 = args[1].parse()?;
    let seed2: u64 = args[2].parse()?;

    let mut rng1 = SmallRng::seed_from_u64(seed1);
    let mut rng2 = SmallRng::seed_from_u64(seed2);

    let mut p1 = AiPlayer::new();
    let mut p2 = AiPlayer::new();
    let mut e1 = GameEngine::new();
    let mut e2 = GameEngine::new();

    p1.place_ships(&mut rng1, e1.board_mut()).map_err(|e| anyhow::anyhow!(e))?;
    p2.place_ships(&mut rng2, e2.board_mut()).map_err(|e| anyhow::anyhow!(e))?;

    let (t1, t2) = InMemoryTransport::pair();

    let f1 = async move {
        let mut node = PlayerNode::new(Box::new(p1), e1, Box::new(t1));
        node.run(&mut rng1, true).await?;
        Ok::<(GameStatus, usize), anyhow::Error>((node.status(), node.guess_count()))
    };

    let f2 = async move {
        let mut node = PlayerNode::new(Box::new(p2), e2, Box::new(t2));
        node.run(&mut rng2, false).await?;
        Ok::<(GameStatus, usize), anyhow::Error>((node.status(), node.guess_count()))
    };

    let (res1, res2) = tokio::try_join!(f1, f2)?;

    let winner = match (res1.0, res2.0) {
        (GameStatus::Won, GameStatus::Lost) => Some("player1"),
        (GameStatus::Lost, GameStatus::Won) => Some("player2"),
        _ => None,
    };

    let result = json!({
        "player1": {"status": format!("{:?}", res1.0), "guesses": res1.1},
        "player2": {"status": format!("{:?}", res2.0), "guesses": res2.1},
        "winner": winner,
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

