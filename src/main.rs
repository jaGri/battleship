#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
use battleship::{
    calc_pdf, print_player_view, print_probability_board, ship_name_static,
    transport::in_memory::InMemoryTransport, transport::tcp::TcpTransport,
    HeartbeatTransport, AiPlayer, CliPlayer, GameEngine, GameStatus, Player, PlayerNode,
    PROTOCOL_VERSION,
};

#[cfg(feature = "std")]
use clap::{Parser, ValueEnum};
#[cfg(feature = "std")]
use rand::rngs::SmallRng;
#[cfg(feature = "std")]
use rand::SeedableRng;
#[cfg(feature = "std")]
use tokio::net::TcpListener;
#[cfg(feature = "std")]
use tokio::time::Duration;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[cfg(feature = "std")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
#[cfg(feature = "std")]
enum PlayerType {
    Human,
    Ai,
}

#[derive(Parser)]
#[cfg(feature = "std")]
enum Commands {
    /// Play against an AI on the local machine.
    Local {
        #[arg(long, help = "Fix RNG seed for reproducible games (e.g., --seed 12345)")]
        seed: Option<u64>,
    },
    /// Host a networked game and wait for a client to connect.
    TcpServer {
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
        #[arg(long, value_enum, default_value_t = PlayerType::Human)]
        player: PlayerType,
        #[arg(long, help = "Fix RNG seed for reproducible games (e.g., --seed 12345)")]
        seed: Option<u64>,
    },
    /// Connect to a networked game hosted by a server.
    TcpClient {
        #[arg(long, default_value = "127.0.0.1:8080")]
        connect: String,
        #[arg(long, value_enum, default_value_t = PlayerType::Human)]
        player: PlayerType,
        #[arg(long, help = "Fix RNG seed for reproducible games (e.g., --seed 12345)")]
        seed: Option<u64>,
    },
}

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Local { seed } => {
            println!("Starting local AI vs AI game...");
            if let Some(s) = seed {
                println!("Using fixed seed: {} (game will be reproducible)", s);
            }
            let mut rng1 = if let Some(s) = seed {
                SmallRng::seed_from_u64(s)
            } else {
                let mut seed_rng = rand::rng();
                SmallRng::from_rng(&mut seed_rng)
            };
            let mut rng2 = if let Some(s) = seed {
                SmallRng::seed_from_u64(s.wrapping_add(1))
            } else {
                let mut seed_rng = rand::rng();
                SmallRng::from_rng(&mut seed_rng)
            };

            let mut ai1 = AiPlayer::new();
            let mut ai2 = AiPlayer::new();
            let mut engine1 = GameEngine::new();
            let mut engine2 = GameEngine::new();

            ai1.place_ships(&mut rng1, engine1.board_mut())
                .map_err(|e| anyhow::anyhow!(e))?;
            ai2.place_ships(&mut rng2, engine2.board_mut())
                .map_err(|e| anyhow::anyhow!(e))?;

            let (t1, t2) = InMemoryTransport::pair();
            let transport1 = Box::new(HeartbeatTransport::disabled(t1));
            let transport2 = Box::new(HeartbeatTransport::disabled(t2));

            let ai1_future = async move {
                let mut node = PlayerNode::new(Box::new(ai1), engine1, transport1);
                node.run(&mut rng1, true).await
            };

            let ai2_future = async move {
                let mut node = PlayerNode::new(Box::new(ai2), engine2, transport2);
                node.run(&mut rng2, false).await
            };

            tokio::try_join!(ai1_future, ai2_future)?;
        }
        Commands::TcpServer { bind, player, seed } => {
            println!("Starting TCP server at {}...", bind);
            if let Some(s) = seed {
                println!("Using fixed seed: {} (game will be reproducible)", s);
            }
            let listener = TcpListener::bind(&bind).await?;
            println!("Waiting for a player to connect...");
            let (stream, addr) = listener.accept().await?;
            println!("Player connected from {}", addr);

            let transport = Box::new(HeartbeatTransport::new(
                TcpTransport::new(stream),
                Duration::from_secs(10),
                Duration::from_secs(45),
            ));
            let mut rng = if let Some(s) = seed {
                SmallRng::seed_from_u64(s)
            } else {
                let mut seed_rng = rand::rng();
                SmallRng::from_rng(&mut seed_rng)
            };
            let mut engine = GameEngine::new();

            match player {
                PlayerType::Human => {
                    let mut cli_player = CliPlayer::new();
                    cli_player
                        .place_ships(&mut rng, engine.board_mut())
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let game_future = run_cli(cli_player, engine, transport, rng, true);
                    if let Err(e) = game_future.await {
                        eprintln!("Game ended with an error: {}", e);
                    }
                }
                PlayerType::Ai => {
                    println!("AI player selected.");
                    let mut ai_player = AiPlayer::new();
                    ai_player
                        .place_ships(&mut rng, engine.board_mut())
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let mut node = PlayerNode::new(Box::new(ai_player), engine, transport);
                    let game_future = node.run(&mut rng, true);
                    if let Err(e) = game_future.await {
                        eprintln!("Game ended with an error: {}", e);
                    }
                }
            }
        }
        Commands::TcpClient { connect, player, seed } => {
            println!("Connecting to TCP server at {}...", connect);
            if let Some(s) = seed {
                println!("Using fixed seed: {} (game will be reproducible)", s);
            }
            let tcp = TcpTransport::connect(&connect).await?;
            let transport = Box::new(HeartbeatTransport::new(
                tcp,
                Duration::from_secs(10),
                Duration::from_secs(45),
            ));
            println!("Connected successfully!");

            let mut rng = if let Some(s) = seed {
                SmallRng::seed_from_u64(s)
            } else {
                let mut seed_rng = rand::rng();
                SmallRng::from_rng(&mut seed_rng)
            };
            let mut engine = GameEngine::new();

            match player {
                PlayerType::Human => {
                    let mut cli_player = CliPlayer::new();
                    cli_player
                        .place_ships(&mut rng, engine.board_mut())
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let game_future = run_cli(cli_player, engine, transport, rng, false);
                    if let Err(e) = game_future.await {
                        eprintln!("Game ended with an error: {}", e);
                    }
                }
                PlayerType::Ai => {
                    println!("AI player selected.");
                    let mut ai_player = AiPlayer::new();
                    ai_player
                        .place_ships(&mut rng, engine.board_mut())
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let mut node = PlayerNode::new(Box::new(ai_player), engine, transport);
                    let game_future = node.run(&mut rng, false);
                    if let Err(e) = game_future.await {
                        eprintln!("Game ended with an error: {}", e);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "std")]
async fn run_cli(
    mut player: CliPlayer,
    mut engine: GameEngine,
    mut transport: Box<dyn battleship::transport::Transport>,
    mut rng: SmallRng,
    first_move: bool,
) -> anyhow::Result<()> {
    // Perform handshake
    if first_move {
        // Send handshake
        transport
            .send(battleship::Message::Handshake {
                version: PROTOCOL_VERSION,
            })
            .await?;
        
        // Wait for ack
        let reply = transport.recv().await?;
        match reply {
            battleship::Message::HandshakeAck { version } if version == PROTOCOL_VERSION => {}
            battleship::Message::HandshakeAck { version } => {
                return Err(anyhow::anyhow!(
                    "Protocol version mismatch: expected {}, got {}",
                    PROTOCOL_VERSION,
                    version
                ));
            }
            _ => return Err(anyhow::anyhow!("Expected HandshakeAck")),
        }
    } else {
        // Wait for handshake
        let msg = transport.recv().await?;
        match msg {
            battleship::Message::Handshake { version } if version == PROTOCOL_VERSION => {
                // Send ack
                transport
                    .send(battleship::Message::HandshakeAck {
                        version: PROTOCOL_VERSION,
                    })
                    .await?;
            }
            battleship::Message::Handshake { version } => {
                return Err(anyhow::anyhow!(
                    "Protocol version mismatch: expected {}, got {}",
                    PROTOCOL_VERSION,
                    version
                ));
            }
            _ => return Err(anyhow::anyhow!("Expected Handshake")),
        }
    }
    
    let mut my_turn = first_move;
    let mut my_seq: u64 = 0;
    let mut expected_recv_seq: u64 = 0;
    loop {
        if my_turn {
            std::println!("\n╔══════════════════════════════════════════════════════════╗");
            std::println!("║                     YOUR TURN                            ║");
            std::println!("╚══════════════════════════════════════════════════════════╝");
            print_player_view(&engine);
            let pdf = calc_pdf(
                &engine.guess_hits(),
                &engine.guess_misses(),
                &engine.enemy_ship_lengths_remaining(),
            );
            print_probability_board(&pdf);

            let (r, c) = player.select_target(
                &mut rng,
                &engine.guess_hits(),
                &engine.guess_misses(),
                &engine.enemy_ship_lengths_remaining(),
            );
            transport
                .send(battleship::Message::Guess {
                    version: PROTOCOL_VERSION,
                    seq: my_seq,
                    x: r as u8,
                    y: c as u8,
                })
                .await?;
            let reply = transport.recv().await?;
            let res_domain = match reply {
                battleship::Message::StatusResp {
                    version,
                    seq: resp_seq,
                    res,
                } if resp_seq == my_seq => {
                    // Validate version
                    if version != PROTOCOL_VERSION {
                        return Err(anyhow::anyhow!(
                            "Protocol version mismatch in response: expected {}, got {}",
                            PROTOCOL_VERSION,
                            version
                        ));
                    }
                    res
                }
                battleship::Message::StatusResp {
                    seq: resp_seq, ..
                } => {
                    return Err(anyhow::anyhow!(
                        "Sequence mismatch: expected {}, got {}",
                        my_seq,
                        resp_seq
                    ))
                }
                _ => return Err(anyhow::anyhow!("unexpected reply")),
            };
            my_seq += 1;
            let res_common = match res_domain {
                battleship::domain::GuessResult::Hit => battleship::GuessResult::Hit,
                battleship::domain::GuessResult::Miss => battleship::GuessResult::Miss,
                battleship::domain::GuessResult::Sink(name) => {
                    let static_name =
                        ship_name_static(&name).ok_or_else(|| anyhow::anyhow!("unknown ship"))?;
                    battleship::GuessResult::Sink(static_name)
                }
            };
            engine
                .record_guess(r, c, res_common)
                .map_err(|e| anyhow::anyhow!(e))?;
            player.handle_guess_result((r, c), res_common);
            my_turn = false;
        } else {
            println!("Waiting for opponent's move...");
            let msg = transport.recv().await?;
            match msg {
                battleship::Message::Guess {
                    version,
                    seq: msg_seq,
                    x,
                    y,
                } => {
                    // Validate version
                    if version != PROTOCOL_VERSION {
                        return Err(anyhow::anyhow!(
                            "Protocol version mismatch in guess: expected {}, got {}",
                            PROTOCOL_VERSION,
                            version
                        ));
                    }
                    
                    // Validate sequence number
                    if msg_seq != expected_recv_seq {
                        return Err(anyhow::anyhow!(
                            "Out-of-order message: expected seq {}, got {}",
                            expected_recv_seq,
                            msg_seq
                        ));
                    }
                    
                    let res_common = engine
                        .opponent_guess(x as usize, y as usize)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    player.handle_opponent_guess((x as usize, y as usize), res_common);
                    let res_domain = battleship::domain::GuessResult::from(res_common);
                    transport
                        .send(battleship::Message::StatusResp {
                            version: PROTOCOL_VERSION,
                            seq: msg_seq,
                            res: res_domain,
                        })
                        .await?;
                    expected_recv_seq += 1;
                }
                _ => return Err(anyhow::anyhow!("unexpected message received")),
            }
            my_turn = true;
        }

        if !matches!(engine.status(), GameStatus::InProgress) {
            break;
        }
    }
    std::println!("\n╔══════════════════════════════════════════════════════════╗");
    std::println!("║                   GAME OVER                              ║");
    std::println!("╚══════════════════════════════════════════════════════════╝\n");
    print_player_view(&engine);
    match engine.status() {
        GameStatus::Won => {
            std::println!("\n🎉🎉🎉 VICTORY! 🎉🎉🎉");
            std::println!("You have sunk all enemy ships!");
        }
        GameStatus::Lost => {
            std::println!("\n💀 DEFEAT 💀");
            std::println!("All your ships have been destroyed.");
        }
        _ => {}
    }
    Ok(())
}
