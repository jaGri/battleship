use battleship::transport::tcp::TcpTransport;
use battleship::transport::Transport;
use battleship::protocol::{Message, PROTOCOL_VERSION, GameApi};
use battleship::domain::{GuessResult, GameStatus, Ship, SyncPayload};
use battleship::{GameEngine, Skeleton, Stub};
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;

struct TestEngine {
    engine: Arc<Mutex<GameEngine>>,
}

impl TestEngine {
    fn new(engine: GameEngine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
        }
    }
}

#[async_trait::async_trait]
impl GameApi for TestEngine {
    async fn make_guess(&mut self, x: u8, y: u8) -> anyhow::Result<GuessResult> {
        let mut eng = self.engine.lock().await;
        let res = eng.opponent_guess(x as usize, y as usize)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(GuessResult::from(res))
    }
    
    async fn get_ship_status(&self, ship_id: usize) -> anyhow::Result<Ship> {
        let eng = self.engine.lock().await;
        eng.get_ship_status(ship_id).await
    }
    
    async fn sync_state(&mut self, payload: SyncPayload) -> anyhow::Result<()> {
        let mut eng = self.engine.lock().await;
        eng.sync_state(payload).await
    }
    
    fn status(&self) -> GameStatus {
        GameStatus::InProgress
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_disconnect_reconnect_basic() -> anyhow::Result<()> {
    // Start a server
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    // Create game engines
    let mut engine1 = GameEngine::new();
    let mut engine2 = GameEngine::new();
    
    // Place ships
    use rand::{rngs::SmallRng, SeedableRng};
    let mut rng = SmallRng::seed_from_u64(42);
    for i in 0..battleship::NUM_SHIPS as usize {
        let (r, c, o) = engine1.board_mut().random_placement(&mut rng, i)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        engine1.board_mut().place(i, r, c, o)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let (r, c, o) = engine2.board_mut().random_placement(&mut rng, i)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        engine2.board_mut().place(i, r, c, o)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    
    // Save initial state
    let state1 = engine1.state();
    
    // Start server
    let server_engine = TestEngine::new(engine2);
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let transport = TcpTransport::new(socket);
        let mut skeleton = Skeleton::new(server_engine, transport);
        skeleton.run().await
    });
    
    // Connect client
    let stream = TcpTransport::connect(addr).await?;
    let mut stub = Stub::new(stream);
    
    // Make a guess
    let res = stub.make_guess(0, 0).await?;
    assert!(matches!(res, GuessResult::Hit | GuessResult::Miss | GuessResult::Sink(_)));
    
    // Disconnect (drop the stub)
    drop(stub);
    
    // Wait a bit
    sleep(Duration::from_millis(100)).await;
    
    // Reconnect
    let stream2 = TcpTransport::connect(addr).await?;
    let mut stub2 = Stub::new(stream2);
    
    // Sync state
    let sync_payload = SyncPayload {
        game_state: state1,
        enemy_ships_remaining: [true; 5],
    };
    stub2.sync_state(sync_payload).await?;
    
    // Make another guess after reconnect
    let res2 = stub2.make_guess(1, 1).await?;
    assert!(matches!(res2, GuessResult::Hit | GuessResult::Miss | GuessResult::Sink(_)));
    
    drop(stub2);
    
    // Server should eventually finish
    let _ = tokio::time::timeout(Duration::from_secs(1), server_task).await;
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_disconnect_during_handshake() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(socket);
        
        // Wait for handshake but don't send response
        let msg = transport.recv().await.unwrap();
        assert!(matches!(msg, Message::Handshake { .. }));
        
        // Simulate disconnect without responding
        drop(transport);
    });
    
    // Client connects
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Send handshake
    transport.send(Message::Handshake { version: PROTOCOL_VERSION }).await?;
    
    // Try to receive response - should fail due to disconnect
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        transport.recv()
    ).await;
    
    assert!(result.is_err() || result.unwrap().is_err());
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reconnect_with_state_sync() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    // Create and initialize engine
    let mut engine = GameEngine::new();
    use rand::{rngs::SmallRng, SeedableRng};
    let mut rng = SmallRng::seed_from_u64(123);
    for i in 0..battleship::NUM_SHIPS as usize {
        let (r, c, o) = engine.board_mut().random_placement(&mut rng, i)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        engine.board_mut().place(i, r, c, o)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    
    // Make some guesses
    engine.record_guess(0, 0, battleship::GuessResult::Miss)?;
    engine.record_guess(1, 1, battleship::GuessResult::Hit)?;
    
    let state_before_disconnect = engine.state();
    
    // First connection
    let engine_clone = TestEngine::new(GameEngine::new());
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let transport = TcpTransport::new(socket);
        let mut skeleton = Skeleton::new(engine_clone, transport);
        let _ = skeleton.run().await;
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut stub = Stub::new(stream);
    
    // Make a guess
    let _ = stub.make_guess(2, 2).await;
    
    // Disconnect
    drop(stub);
    
    sleep(Duration::from_millis(50)).await;
    
    // Reconnect with new server
    let listener2 = TcpListener::bind("127.0.0.1:0").await?;
    let addr2 = listener2.local_addr()?;
    
    let mut engine2 = GameEngine::new();
    for i in 0..battleship::NUM_SHIPS as usize {
        let (r, c, o) = engine2.board_mut().random_placement(&mut rng, i)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        engine2.board_mut().place(i, r, c, o)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }
    
    let engine2_clone = TestEngine::new(engine2);
    let server_task2 = tokio::spawn(async move {
        let (socket, _) = listener2.accept().await.unwrap();
        let transport = TcpTransport::new(socket);
        let mut skeleton = Skeleton::new(engine2_clone, transport);
        let _ = skeleton.run().await;
    });
    
    let stream2 = TcpTransport::connect(addr2).await?;
    let mut stub2 = Stub::new(stream2);
    
    // Sync state from before disconnect
    let sync_payload = SyncPayload {
        game_state: state_before_disconnect,
        enemy_ships_remaining: [true; 5],
    };
    stub2.sync_state(sync_payload).await?;
    
    // Verify we can continue playing
    let res = stub2.make_guess(3, 3).await?;
    assert!(matches!(res, GuessResult::Hit | GuessResult::Miss | GuessResult::Sink(_)));
    
    drop(stub2);
    let _ = tokio::time::timeout(Duration::from_millis(500), server_task).await;
    let _ = tokio::time::timeout(Duration::from_millis(500), server_task2).await;
    
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_abrupt_disconnect() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut transport = TcpTransport::new(socket);
        
        // Complete handshake
        let msg = transport.recv().await.unwrap();
        assert!(matches!(msg, Message::Handshake { .. }));
        transport.send(Message::HandshakeAck { version: PROTOCOL_VERSION }).await.unwrap();
        
        // Wait for message but connection will drop
        let result = transport.recv().await;
        assert!(result.is_err());
    });
    
    let stream = TcpTransport::connect(addr).await?;
    let mut transport = stream;
    
    // Handshake
    transport.send(Message::Handshake { version: PROTOCOL_VERSION }).await?;
    let msg = transport.recv().await?;
    assert!(matches!(msg, Message::HandshakeAck { .. }));
    
    // Abruptly drop connection without graceful shutdown
    drop(transport);
    
    server_task.await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_disconnect_reconnect_cycles() -> anyhow::Result<()> {
    use rand::{rngs::SmallRng, SeedableRng};
    
    for cycle in 0..3 {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        
        let mut rng = SmallRng::seed_from_u64(cycle);
        let mut engine = GameEngine::new();
        for i in 0..battleship::NUM_SHIPS as usize {
            let (r, c, o) = engine.board_mut().random_placement(&mut rng, i)
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
            engine.board_mut().place(i, r, c, o)
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        
        let test_engine = TestEngine::new(engine);
        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let transport = TcpTransport::new(socket);
            let mut skeleton = Skeleton::new(test_engine, transport);
            let _ = skeleton.run().await;
        });
        
        let stream = TcpTransport::connect(addr).await?;
        let mut stub = Stub::new(stream);
        
        // Make a guess
        let _ = stub.make_guess(cycle as u8, cycle as u8).await;
        
        // Disconnect
        drop(stub);
        
        sleep(Duration::from_millis(50)).await;
        let _ = tokio::time::timeout(Duration::from_millis(500), server_task).await;
    }
    
    Ok(())
}
