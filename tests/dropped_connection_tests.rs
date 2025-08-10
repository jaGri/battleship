use battleship::domain::{GameStatus, GuessResult, Ship, SyncPayload};
use battleship::protocol::GameApi;
use battleship::transport::in_memory::InMemoryTransport;
use battleship::{Skeleton, Stub};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone)]
struct DummyEngine {
    resyncs: Arc<AtomicUsize>,
}

impl DummyEngine {
    fn new() -> Self {
        Self {
            resyncs: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl GameApi for DummyEngine {
    async fn make_guess(&mut self, _x: u8, _y: u8) -> anyhow::Result<GuessResult> {
        Ok(GuessResult::Hit)
    }

    async fn get_ship_status(&self, _ship_id: usize) -> anyhow::Result<Ship> {
        Ok(Ship {
            name: "dummy".into(),
            sunk: false,
            position: None,
        })
    }

    async fn sync_state(&mut self, _payload: SyncPayload) -> anyhow::Result<()> {
        Ok(())
    }

    async fn resync(&mut self, _state: SyncPayload) -> anyhow::Result<()> {
        self.resyncs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn status(&self) -> GameStatus {
        GameStatus::InProgress
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reconnect_and_resync() -> anyhow::Result<()> {
    // Initial connection
    let (server_transport, client_transport) = InMemoryTransport::pair();
    let engine = DummyEngine::new();
    let engine_clone = engine.clone();

    let server = tokio::spawn(async move {
        let mut skeleton = Skeleton::new(engine_clone, server_transport);
        skeleton.run().await.unwrap();
    });

    let mut stub = Stub::new(client_transport);
    stub.make_guess(1, 1).await?;

    // Drop connection to simulate network failure
    drop(stub);
    server.await.unwrap();

    // Reconnect with new transports
    let (server_transport, client_transport) = InMemoryTransport::pair();
    let engine_clone = engine.clone();
    let server = tokio::spawn(async move {
        let mut skeleton = Skeleton::new(engine_clone, server_transport);
        skeleton.run().await.unwrap();
    });

    let mut stub = Stub::new(client_transport);
    stub.resync(SyncPayload).await?;
    assert_eq!(engine.resyncs.load(Ordering::SeqCst), 1);

    let res = stub.make_guess(2, 2).await?;
    assert!(matches!(res, GuessResult::Hit));

    drop(stub);
    server.await.unwrap();
    Ok(())
}
