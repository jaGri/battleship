use battleship::{AiPlayer, GameEngine, GameStatus, PlayerNode, Player};
use battleship::transport::{Transport};
use battleship::transport::in_memory::InMemoryTransport;
use battleship::protocol::Message;
use rand::rngs::SmallRng;
use rand::SeedableRng;

#[tokio::test(flavor = "multi_thread")]
async fn test_ai_transport_game() -> anyhow::Result<()> {
    let (mut t1, mut t2) = InMemoryTransport::pair();

    // Send an initial guess so one node can start processing immediately
    t2.send(Message::Guess { x: 0, y: 0 }).await?;

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

    tokio::join!(
        async { node1.run(&mut rng1).await.unwrap(); },
        async { node2.run(&mut rng2).await.unwrap(); },
    );

    let status1 = node1.status();
    let status2 = node2.status();
    let turns = node1.guess_count().max(node2.guess_count());
    let winner = if matches!(status1, GameStatus::Won) { "Player 1" } else { "Player 2" };

    println!("{} wins after {} turns", winner, turns);

    assert!(matches!(status1, GameStatus::Won | GameStatus::Lost));
    assert!(matches!(status2, GameStatus::Won | GameStatus::Lost));
    Ok(())
}
