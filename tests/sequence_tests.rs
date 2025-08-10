use battleship::domain::{GameStatus, GuessResult, Ship, SyncPayload};
use battleship::protocol::GameApi;
use battleship::transport::in_memory::InMemoryTransport;
use battleship::transport::Transport;
use battleship::{Message, Skeleton, PROTOCOL_VERSION};

struct DummyEngine;

#[async_trait::async_trait]
impl GameApi for DummyEngine {
    async fn make_guess(&mut self, _x: u8, _y: u8) -> anyhow::Result<GuessResult> {
        Ok(GuessResult::Hit)
    }
    async fn get_ship_status(&self, _ship_id: usize) -> anyhow::Result<Ship> {
        Ok(Ship {
            name: "dummy".to_string(),
            sunk: false,
            position: None,
        })
    }
    async fn sync_state(&mut self, _payload: SyncPayload) -> anyhow::Result<()> {
        Ok(())
    }
    fn status(&self) -> GameStatus {
        GameStatus::InProgress
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_out_of_order_message() -> anyhow::Result<()> {
    let (server_transport, mut client_transport) = InMemoryTransport::pair();
    let server = tokio::spawn(async move {
        let engine = DummyEngine;
        let mut skeleton = Skeleton::new(engine, server_transport);
        skeleton.run().await.unwrap();
    });

    // Send out-of-order message (seq 1 when 0 expected)
    client_transport
        .send(Message::Guess {
            version: PROTOCOL_VERSION,
            seq: 1,
            x: 0,
            y: 0,
        })
        .await?;
    match client_transport.recv().await? {
        Message::Ack { seq, .. } => assert_eq!(seq, 1),
        _ => panic!("expected Ack"),
    }

    // Send correct sequence
    client_transport
        .send(Message::Guess {
            version: PROTOCOL_VERSION,
            seq: 0,
            x: 0,
            y: 0,
        })
        .await?;
    match client_transport.recv().await? {
        Message::StatusResp { seq, res, .. } => {
            assert_eq!(seq, 0);
            assert!(matches!(res, GuessResult::Hit));
        }
        _ => panic!("expected StatusResp"),
    }

    drop(client_transport);
    server.await.unwrap();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_duplicate_message() -> anyhow::Result<()> {
    let (server_transport, mut client_transport) = InMemoryTransport::pair();
    let server = tokio::spawn(async move {
        let engine = DummyEngine;
        let mut skeleton = Skeleton::new(engine, server_transport);
        skeleton.run().await.unwrap();
    });

    client_transport
        .send(Message::Guess {
            version: PROTOCOL_VERSION,
            seq: 0,
            x: 0,
            y: 0,
        })
        .await?;
    match client_transport.recv().await? {
        Message::StatusResp { seq, .. } => assert_eq!(seq, 0),
        _ => panic!("expected StatusResp"),
    }

    // Send duplicate seq 0
    client_transport
        .send(Message::Guess {
            version: PROTOCOL_VERSION,
            seq: 0,
            x: 1,
            y: 1,
        })
        .await?;
    match client_transport.recv().await? {
        Message::Ack { seq, .. } => assert_eq!(seq, 0),
        _ => panic!("expected Ack"),
    }

    drop(client_transport);
    server.await.unwrap();
    Ok(())
}
