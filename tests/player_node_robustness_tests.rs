/// Tests for PlayerNode robustness improvements:
/// - Explicit handling of unexpected messages
/// - Logging of mismatched seq/version
/// - Session closure on errors instead of continuing
use battleship::transport::in_memory::InMemoryTransport;
use battleship::transport::Transport;
use battleship::{AiPlayer, GameEngine, Message, Player, PlayerNode, PROTOCOL_VERSION};
use battleship::domain::GuessResult;
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[tokio::test]
async fn test_handshake_rejects_wrong_version() {
    let (t1, mut t2) = InMemoryTransport::pair();
    let mut rng = SmallRng::seed_from_u64(42);
    
    // Set up node with placed ships
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t1));

    // Start node as initiator in background
    let node_handle = tokio::spawn(async move {
        node.run(&mut rng, true).await
    });

    // Receive handshake
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { version: PROTOCOL_VERSION }));

    // Send back wrong version
    t2.send(Message::HandshakeAck {
        version: PROTOCOL_VERSION + 1,
    })
    .await
    .unwrap();

    // Node should reject and return error
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("version mismatch"));
}

#[tokio::test]
async fn test_handshake_rejects_unexpected_message() {
    let (t1, mut t2) = InMemoryTransport::pair();
    let mut rng = SmallRng::seed_from_u64(123);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t1));

    // Start node as initiator
    let node_handle = tokio::spawn(async move {
        node.run(&mut rng, true).await
    });

    // Receive handshake
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { .. }));

    // Send unexpected message instead of HandshakeAck
    t2.send(Message::Guess {
        version: PROTOCOL_VERSION,
        seq: 0,
        x: 0,
        y: 0,
    })
    .await
    .unwrap();

    // Node should reject and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Expected HandshakeAck"));
}

#[tokio::test]
async fn test_responder_handshake_rejects_wrong_version() {
    let (mut t1, t2) = InMemoryTransport::pair();
    let mut rng = SmallRng::seed_from_u64(456);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t2));

    // Start node as responder
    let node_handle = tokio::spawn(async move {
        node.run(&mut rng, false).await
    });

    // Send handshake with wrong version
    t1.send(Message::Handshake {
        version: PROTOCOL_VERSION + 1,
    })
    .await
    .unwrap();

    // Node should reject
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("version mismatch"));
}

#[tokio::test]
async fn test_responder_handshake_rejects_unexpected_message() {
    let (mut t1, t2) = InMemoryTransport::pair();
    let mut rng = SmallRng::seed_from_u64(789);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t2));

    // Start node as responder
    let node_handle = tokio::spawn(async move {
        node.run(&mut rng, false).await
    });

    // Send unexpected message instead of Handshake
    t1.send(Message::StatusResp {
        version: PROTOCOL_VERSION,
        seq: 0,
        res: GuessResult::Miss,
    })
    .await
    .unwrap();

    // Node should reject
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Expected Handshake"));
}

#[tokio::test]
async fn test_sequence_mismatch_in_status_resp() {
    let (t1, mut t2) = InMemoryTransport::pair();
    let mut rng1 = SmallRng::seed_from_u64(111);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng1, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t1));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng1, true).await
    });

    // Complete handshake
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { .. }));
    t2.send(Message::HandshakeAck { version: PROTOCOL_VERSION }).await.unwrap();

    // Receive first guess
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Guess { seq: 0, .. }));

    // Send response with wrong sequence number
    t2.send(Message::StatusResp {
        version: PROTOCOL_VERSION,
        seq: 999, // Wrong seq, should be 0
        res: GuessResult::Miss,
    })
    .await
    .unwrap();

    // Node should detect mismatch and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Sequence mismatch"));
}

#[tokio::test]
async fn test_version_mismatch_in_status_resp() {
    let (t1, mut t2) = InMemoryTransport::pair();
    let mut rng1 = SmallRng::seed_from_u64(222);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng1, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t1));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng1, true).await
    });

    // Complete handshake
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { .. }));
    t2.send(Message::HandshakeAck { version: PROTOCOL_VERSION }).await.unwrap();

    // Receive first guess
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Guess { seq: 0, .. }));

    // Send response with wrong version
    t2.send(Message::StatusResp {
        version: PROTOCOL_VERSION + 1,
        seq: 0,
        res: GuessResult::Miss,
    })
    .await
    .unwrap();

    // Node should detect version mismatch and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("version mismatch"));
}

#[tokio::test]
async fn test_unexpected_message_instead_of_status_resp() {
    let (t1, mut t2) = InMemoryTransport::pair();
    let mut rng1 = SmallRng::seed_from_u64(333);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng1, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t1));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng1, true).await
    });

    // Complete handshake
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Handshake { .. }));
    t2.send(Message::HandshakeAck { version: PROTOCOL_VERSION }).await.unwrap();

    // Receive first guess
    let msg = t2.recv().await.unwrap();
    assert!(matches!(msg, Message::Guess { seq: 0, .. }));

    // Send completely unexpected message type
    t2.send(Message::Heartbeat { version: PROTOCOL_VERSION })
        .await
        .unwrap();

    // Node should reject and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Expected StatusResp"));
}

#[tokio::test]
async fn test_sequence_mismatch_in_guess() {
    let (mut t1, t2) = InMemoryTransport::pair();
    let mut rng2 = SmallRng::seed_from_u64(444);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng2, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t2));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng2, false).await
    });

    // Complete handshake
    t1.send(Message::Handshake { version: PROTOCOL_VERSION }).await.unwrap();
    let msg = t1.recv().await.unwrap();
    assert!(matches!(msg, Message::HandshakeAck { .. }));

    // Send guess with wrong sequence number
    t1.send(Message::Guess {
        version: PROTOCOL_VERSION,
        seq: 5, // Wrong, should be 0
        x: 0,
        y: 0,
    })
    .await
    .unwrap();

    // Node should detect out-of-order and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Out-of-order"));
}

#[tokio::test]
async fn test_version_mismatch_in_guess() {
    let (mut t1, t2) = InMemoryTransport::pair();
    let mut rng2 = SmallRng::seed_from_u64(555);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng2, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t2));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng2, false).await
    });

    // Complete handshake
    t1.send(Message::Handshake { version: PROTOCOL_VERSION }).await.unwrap();
    let msg = t1.recv().await.unwrap();
    assert!(matches!(msg, Message::HandshakeAck { .. }));

    // Send guess with wrong version
    t1.send(Message::Guess {
        version: PROTOCOL_VERSION + 1,
        seq: 0,
        x: 0,
        y: 0,
    })
    .await
    .unwrap();

    // Node should detect version mismatch and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("version mismatch"));
}

#[tokio::test]
async fn test_unexpected_message_instead_of_guess() {
    let (mut t1, t2) = InMemoryTransport::pair();
    let mut rng2 = SmallRng::seed_from_u64(666);
    let mut player = AiPlayer::new();
    let mut engine = GameEngine::new();
    player.place_ships(&mut rng2, engine.board_mut()).unwrap();
    let mut node = PlayerNode::new(Box::new(player), engine, Box::new(t2));

    let node_handle = tokio::spawn(async move {
        node.run(&mut rng2, false).await
    });

    // Complete handshake
    t1.send(Message::Handshake { version: PROTOCOL_VERSION }).await.unwrap();
    let msg = t1.recv().await.unwrap();
    assert!(matches!(msg, Message::HandshakeAck { .. }));

    // Send unexpected message type instead of Guess
    t1.send(Message::StatusResp {
        version: PROTOCOL_VERSION,
        seq: 0,
        res: GuessResult::Miss,
    })
    .await
    .unwrap();

    // Node should reject and close session
    let result = node_handle.await.unwrap();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Expected Guess"));
}

#[tokio::test]
async fn test_successful_game_with_strict_validation() {
    // This test verifies that valid games still work correctly with all the new validation
    let (t1, t2) = InMemoryTransport::pair();
    let mut rng1 = SmallRng::seed_from_u64(1001);
    let mut rng2 = SmallRng::seed_from_u64(1002);

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

    // Both should complete successfully
    assert!(result.0.is_ok(), "Player 1 failed: {:?}", result.0);
    assert!(result.1.is_ok(), "Player 2 failed: {:?}", result.1);
    
    // One should have won, one should have lost
    let status1 = node1.status();
    let status2 = node2.status();
    assert_ne!(status1, status2);
}
