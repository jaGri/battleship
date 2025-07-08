use battleship::{Skeleton, TcpTransport, GameApi, Message, GuessResult, SyncPayload};

struct DummyEngine;

#[async_trait::async_trait]
impl GameApi for DummyEngine {
    async fn make_guess(&mut self, _x: u8, _y: u8) -> anyhow::Result<GuessResult> {
        Ok(GuessResult::Hit)
    }
    async fn sync_state(&mut self, _payload: SyncPayload) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn skeleton_tcp_exchange() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut skel = Skeleton::new(DummyEngine, TcpTransport::from_stream(stream));
        skel.run().await.unwrap();
    });

    let client = async {
        let transport = TcpTransport::connect(&addr.to_string()).await?;
        let mut skel = Skeleton::new(DummyEngine, transport);
        let reply = skel.send(Message::Guess { x: 1, y: 1 }).await?;
        assert!(matches!(reply, Message::StatusResp(GuessResult::Hit)));
        Ok::<(), anyhow::Error>(())
    };

    client.await?;
    server.await.unwrap();
    Ok(())
}
