use battleship::transport::tcp::TcpTransport;
use battleship::protocol::GameApi;
use battleship::domain::{GuessResult, GameStatus, Ship, SyncPayload};
use battleship::{GameState, GuessBoardState, BoardState, BitBoard, Skeleton, Stub};
use tokio::net::TcpListener;

struct DummyEngine;

#[async_trait::async_trait]
impl GameApi for DummyEngine {
    async fn make_guess(&mut self, _x: u8, _y: u8) -> anyhow::Result<GuessResult> {
        Ok(GuessResult::Hit)
    }
    async fn get_ship_status(&self, _ship_id: usize) -> anyhow::Result<Ship> {
        Ok(Ship { name: "dummy".to_string(), sunk: false, position: None })
    }
    async fn sync_state(&mut self, _payload: SyncPayload) -> anyhow::Result<()> {
        Ok(())
    }
    fn status(&self) -> GameStatus {
        GameStatus::InProgress
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_stub_skeleton_tcp() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let transport = TcpTransport::new(socket);
        let engine = DummyEngine;
        let mut skeleton = Skeleton::new(engine, transport);
        skeleton.run().await.unwrap();
    });

    let stream = TcpTransport::connect(addr).await?;
    let mut stub = Stub::new(stream);

    let res = stub.make_guess(1, 2).await?;
    assert!(matches!(res, GuessResult::Hit));

    let ship = stub.get_ship_status(0).await?;
    assert_eq!(ship.name, "dummy");

    // Create a proper sync payload with game state
    let sync_payload = SyncPayload {
        game_state: GameState {
            my_board: BoardState {
                ship_states: [
                    battleship::ShipState::new("Carrier"),
                    battleship::ShipState::new("Battleship"),
                    battleship::ShipState::new("Cruiser"),
                    battleship::ShipState::new("Submarine"),
                    battleship::ShipState::new("Destroyer"),
                ],
                ship_map: BitBoard::<u128, 10>::new(),
                hits: BitBoard::<u128, 10>::new(),
                misses: BitBoard::<u128, 10>::new(),
            },
            my_guesses: GuessBoardState {
                hits: BitBoard::<u128, 10>::new(),
                misses: BitBoard::<u128, 10>::new(),
            },
            enemy_ships_remaining: [true; 5],
            enemy_remaining: 17,
        },
        enemy_ships_remaining: [true; 5],
    };
    stub.sync_state(sync_payload).await?;

    let status = stub.status();
    assert!(matches!(status, GameStatus::InProgress));

    drop(stub);
    server.await.unwrap();
    Ok(())
}
