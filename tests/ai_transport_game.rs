use battleship::transport::in_memory::InMemoryTransport;
use battleship::transport::Transport;
use battleship::transport::tcp::TcpTransport;
use battleship::{AiPlayer, GameEngine, GameStatus, Player, PlayerNode};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use tokio::net::TcpListener;

enum TransportKind {
    InMemory,
    Tcp,
}

async fn run_game(kind: TransportKind) -> anyhow::Result<()> {
    let (t1, t2): (Box<dyn Transport>, Box<dyn Transport>) = match kind {
        TransportKind::InMemory => {
            let (t1, t2) = InMemoryTransport::pair();
            (Box::new(t1), Box::new(t2))
        }
        TransportKind::Tcp => {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let addr = listener.local_addr()?;
            let accept = tokio::spawn(async move {
                let (socket, _) = listener.accept().await?;
                Ok::<TcpTransport, anyhow::Error>(TcpTransport::new(socket))
            });
            let client = TcpTransport::connect(addr).await?;
            let server = accept.await??;
            (Box::new(server), Box::new(client))
        }
    };


    let mut rng1 = SmallRng::seed_from_u64(1);
    let mut rng2 = SmallRng::seed_from_u64(2);

    let mut p1 = AiPlayer::new();
    let mut p2 = AiPlayer::new();
    let mut e1 = GameEngine::new();
    let mut e2 = GameEngine::new();

    p1.place_ships(&mut rng1, e1.board_mut()).unwrap();
    p2.place_ships(&mut rng2, e2.board_mut()).unwrap();

    let mut node1 = PlayerNode::new(Box::new(p1), e1, t1);
    let mut node2 = PlayerNode::new(Box::new(p2), e2, t2);

    tokio::join!(
        async {
            node1.run(&mut rng1, true).await.unwrap();
        },
        async {
            node2.run(&mut rng2, false).await.unwrap();
        },
    );

    let status1 = node1.status();
    let status2 = node2.status();
    let turns = node1.guess_count().max(node2.guess_count());
    let winner = if matches!(status1, GameStatus::Won) {
        "Player 1"
    } else {
        "Player 2"
    };

    println!("{} wins after {} turns", winner, turns);

    assert!(matches!(status1, GameStatus::Won | GameStatus::Lost));
    assert!(matches!(status2, GameStatus::Won | GameStatus::Lost));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ai_transport_game_in_memory() -> anyhow::Result<()> {
    run_game(TransportKind::InMemory).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ai_transport_game_tcp() -> anyhow::Result<()> {
    run_game(TransportKind::Tcp).await
}
