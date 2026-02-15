/// Integration test: Full AI vs AI game with active heartbeat monitoring
use battleship::{
    AiPlayer, GameEngine, PlayerNode, HeartbeatTransport, TcpTransport, Player,
};
use tokio::net::TcpListener;
use tokio::time::Duration;
use rand::SeedableRng;
use rand::rngs::SmallRng;

#[tokio::test]
async fn test_full_game_with_heartbeat() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let tcp = TcpTransport::new(stream);
        let transport = HeartbeatTransport::new(
            tcp,
            Duration::from_millis(500),  // Short interval for testing
            Duration::from_secs(10),
        );

        let mut rng = SmallRng::seed_from_u64(42);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport));
        node.run(&mut rng, true).await
    });

    let client_task = tokio::spawn(async move {
        let tcp = TcpTransport::connect(addr).await.unwrap();
        let transport = HeartbeatTransport::new(
            tcp,
            Duration::from_millis(500),
            Duration::from_secs(10),
        );

        let mut rng = SmallRng::seed_from_u64(43);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport));
        node.run(&mut rng, false).await
    });

    // Both should complete successfully with heartbeats running in background
    let (r1, r2) = tokio::join!(server_task, client_task);
    assert!(r1.unwrap().is_ok(), "Server game should complete successfully");
    assert!(r2.unwrap().is_ok(), "Client game should complete successfully");
}

#[tokio::test]
async fn test_heartbeat_keeps_connection_alive() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let tcp = TcpTransport::new(stream);
        let transport = HeartbeatTransport::new(
            tcp,
            Duration::from_millis(100),  // Frequent heartbeats
            Duration::from_secs(2),       // Short idle timeout
        );

        let mut rng = SmallRng::seed_from_u64(100);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport));
        node.run(&mut rng, true).await
    });

    let client_task = tokio::spawn(async move {
        // Add delay before connecting to simulate network latency
        tokio::time::sleep(Duration::from_millis(50)).await;

        let tcp = TcpTransport::connect(addr).await.unwrap();
        let transport = HeartbeatTransport::new(
            tcp,
            Duration::from_millis(100),
            Duration::from_secs(2),
        );

        let mut rng = SmallRng::seed_from_u64(101);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport));
        node.run(&mut rng, false).await
    });

    // Game should complete despite aggressive idle timeout because heartbeats keep it alive
    let (r1, r2) = tokio::join!(server_task, client_task);
    assert!(r1.unwrap().is_ok(), "Server should complete with heartbeats keeping connection alive");
    assert!(r2.unwrap().is_ok(), "Client should complete with heartbeats keeping connection alive");
}

#[tokio::test]
async fn test_disabled_heartbeat_with_game() {
    use battleship::transport::in_memory::InMemoryTransport;

    let (t1, t2) = InMemoryTransport::pair();
    let transport1 = HeartbeatTransport::disabled(t1);
    let transport2 = HeartbeatTransport::disabled(t2);

    let game1 = tokio::spawn(async move {
        let mut rng = SmallRng::seed_from_u64(200);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport1));
        node.run(&mut rng, true).await
    });

    let game2 = tokio::spawn(async move {
        let mut rng = SmallRng::seed_from_u64(201);
        let mut ai = AiPlayer::new();
        let mut engine = GameEngine::new();
        ai.place_ships(&mut rng, engine.board_mut()).unwrap();

        let mut node = PlayerNode::new(Box::new(ai), engine, Box::new(transport2));
        node.run(&mut rng, false).await
    });

    // Game should complete successfully with disabled heartbeat
    let (r1, r2) = tokio::join!(game1, game2);
    assert!(r1.unwrap().is_ok(), "Game 1 should complete with disabled heartbeat");
    assert!(r2.unwrap().is_ok(), "Game 2 should complete with disabled heartbeat");
}
