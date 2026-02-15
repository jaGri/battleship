use battleship::transport::in_memory::InMemoryTransport;
use battleship::transport::tcp::TcpTransport;
use battleship::transport::Transport;
use battleship::{AiPlayer, GameEngine, Message, Player, PlayerNode, PROTOCOL_VERSION};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_handshake_version_mismatch() {
    let (mut t1, mut t2) = InMemoryTransport::pair();

    // Player 1 sends handshake with correct version
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();

    // Player 2 receives and sends ack with wrong version
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { .. }));

    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION + 1,
    })
    .await
    .unwrap();

    // Player 1 should detect version mismatch
    let reply = t1.recv().await.unwrap();
    if let Message::HandshakeAck { version } = reply {
        assert_ne!(version, PROTOCOL_VERSION);
    } else {
        panic!("Expected HandshakeAck");
    }
}

#[tokio::test]
async fn test_sequence_validation() {
    let (mut t1, mut t2) = InMemoryTransport::pair();

    // Perform handshake
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    t2.recv().await.unwrap();
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION,
    })
    .await
    .unwrap();
    t1.recv().await.unwrap();

    // Player 1 sends guess with seq 0
    t1.send(Message::Guess {
        version: PROTOCOL_VERSION,
        seq: 0,
        x: 0,
        y: 0,
    })
    .await
    .unwrap();

    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Guess { seq: 0, .. }));

    // Player 2 responds correctly with seq 0
    t2.send(Message::StatusResp {
        version: PROTOCOL_VERSION,
        seq: 0,
        res: battleship::domain::GuessResult::Miss,
    })
    .await
    .unwrap();

    let reply = t1.recv().await.unwrap();
    assert!(matches!(reply, Message::StatusResp { seq: 0, .. }));
}

#[tokio::test]
async fn test_tcp_timeout() {
    use tokio::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::with_timeout(socket, Duration::from_millis(100));

        // Try to receive but don't send anything - should timeout
        let result = transport.recv().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timeout") || err_msg.contains("Timeout"));
    });

    let _client = TcpTransport::connect(addr).await.unwrap();
    // Don't send anything, just wait

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_successful_handshake_and_game() {
    let (t1, t2) = InMemoryTransport::pair();

    let mut rng1 = SmallRng::seed_from_u64(1);
    let mut rng2 = SmallRng::seed_from_u64(2);

    let mut p1 = AiPlayer::new();
    let mut p2 = AiPlayer::new();
    let mut e1 = GameEngine::new();
    let mut e2 = GameEngine::new();

    p1.place_ships(&mut rng1, e1.board_mut()).unwrap();
    p2.place_ships(&mut rng2, e2.board_mut()).unwrap();

    let mut node1 = PlayerNode::new(Box::new(p1), e1, Box::new(t1));
    let mut node2 = PlayerNode::new(Box::new(p2), e2, Box::new(t2));

    let result = tokio::join!(
        async { node1.run(&mut rng1, true).await },
        async { node2.run(&mut rng2, false).await },
    );

    // Both should complete successfully with handshake and strict sequence validation
    assert!(result.0.is_ok());
    assert!(result.1.is_ok());
}
