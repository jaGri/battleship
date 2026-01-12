#![cfg(feature = "std")]

use battleship::{
    transport::tcp::TcpTransport, AiPlayer, GameEngine, GameStatus, Player, PlayerNode,
};
use rand::{rngs::SmallRng, SeedableRng};
use tokio::net::TcpListener;

#[tokio::test(flavor = "multi_thread")]
async fn test_ai_vs_ai_tcp_game() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Server Task
    let server_future = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let transport = TcpTransport::new(socket);

        let mut rng = SmallRng::from_seed([0; 32]);
        let mut player = AiPlayer::new();
        let mut engine = GameEngine::new();
        player.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut server_node = PlayerNode::new(Box::new(player), engine, Box::new(transport));
        server_node.run(&mut rng, true).await.unwrap();
        server_node.status()
    });

    // Client Task
    let client_future = tokio::spawn(async move {
        let transport = TcpTransport::connect(addr).await.unwrap();

        let mut rng = SmallRng::from_seed([1; 32]);
        let mut player = AiPlayer::new();
        let mut engine = GameEngine::new();
        player.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut client_node = PlayerNode::new(Box::new(player), engine, Box::new(transport));
        client_node.run(&mut rng, false).await.unwrap();
        client_node.status()
    });

    let (server_status, client_status) = tokio::try_join!(server_future, client_future)?;

    // One must win, one must lose.
    assert!(
        (matches!(server_status, GameStatus::Won) && matches!(client_status, GameStatus::Lost))
            || (matches!(server_status, GameStatus::Lost) && matches!(client_status, GameStatus::Won))
    );

    Ok(())
}
