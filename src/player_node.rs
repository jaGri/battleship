#![cfg(feature = "std")]

extern crate alloc;

use alloc::boxed::Box;
use rand::rngs::SmallRng;

use crate::{
    core::{
        common::GuessResult,
        config::ship_name_static,
        game::GameStatus,
        GameEngine,
    },
    domain::GuessResult as DomainGuessResult,
    player::Player,
    protocol::{Message, PROTOCOL_VERSION},
    transport::Transport,
};

pub struct PlayerNode {
    player: Box<dyn Player>,
    engine: GameEngine,
    transport: Box<dyn Transport>,
}

impl PlayerNode {
    pub fn new(player: Box<dyn Player>, engine: GameEngine, transport: Box<dyn Transport>) -> Self {
        Self {
            player,
            engine,
            transport,
        }
    }

    /// Perform handshake to establish protocol version.
    async fn handshake(&mut self, initiator: bool) -> anyhow::Result<()> {
        if initiator {
            // Send handshake
            self.transport
                .send(Message::Handshake {
                    version: PROTOCOL_VERSION,
                })
                .await?;
            
            // Wait for ack
            let reply = self.transport.recv().await?;
            match reply {
                Message::HandshakeAck { version } if version == PROTOCOL_VERSION => Ok(()),
                Message::HandshakeAck { version } => {
                    eprintln!(
                        "[PlayerNode] Handshake protocol version mismatch: expected {}, peer responded with {}",
                        PROTOCOL_VERSION, version
                    );
                    Err(anyhow::anyhow!(
                        "Protocol version mismatch in HandshakeAck: expected {}, got {}",
                        PROTOCOL_VERSION,
                        version
                    ))
                }
                other => {
                    eprintln!(
                        "[PlayerNode] Expected HandshakeAck, got unexpected message: {:?}",
                        other
                    );
                    Err(anyhow::anyhow!("Expected HandshakeAck, got unexpected message (closing session)"))
                }
            }
        } else {
            // Wait for handshake
            let msg = self.transport.recv().await?;
            match msg {
                Message::Handshake { version } if version == PROTOCOL_VERSION => {
                    // Send ack
                    self.transport
                        .send(Message::HandshakeAck {
                            version: PROTOCOL_VERSION,
                        })
                        .await?;
                    Ok(())
                }
                Message::Handshake { version } => {
                    eprintln!(
                        "[PlayerNode] Handshake protocol version mismatch: expected {}, peer sent {}",
                        PROTOCOL_VERSION, version
                    );
                    Err(anyhow::anyhow!(
                        "Protocol version mismatch in Handshake: expected {}, got {}",
                        PROTOCOL_VERSION,
                        version
                    ))
                }
                other => {
                    eprintln!(
                        "[PlayerNode] Expected Handshake, got unexpected message: {:?}",
                        other
                    );
                    Err(anyhow::anyhow!("Expected Handshake, got unexpected message (closing session)"))
                }
            }
        }
    }

    pub async fn run(&mut self, rng: &mut SmallRng, first_move: bool) -> anyhow::Result<()> {
        // Perform handshake before starting game
        self.handshake(first_move).await?;
        
        let mut my_turn = first_move;
        let mut my_seq: u64 = 0;
        let mut expected_recv_seq: u64 = 0;
        loop {
            if my_turn {
                // Choose our guess and send to opponent
                let (r, c) = self.player.select_target(
                    rng,
                    &self.engine.guess_hits(),
                    &self.engine.guess_misses(),
                    &self.engine.enemy_ship_lengths_remaining(),
                );
                self.transport
                    .send(Message::Guess {
                        version: PROTOCOL_VERSION,
                        seq: my_seq,
                        x: r as u8,
                        y: c as u8,
                    })
                    .await?;
                let reply = self.transport.recv().await?;
                let res_domain = match reply {
                    Message::StatusResp {
                        version,
                        seq: resp_seq,
                        res,
                    } if resp_seq == my_seq && version == PROTOCOL_VERSION => {
                        res
                    }
                    Message::StatusResp {
                        version,
                        seq: resp_seq,
                        ..
                    } => {
                        // Log detailed mismatch information before closing session
                        if version != PROTOCOL_VERSION {
                            eprintln!(
                                "[PlayerNode] Protocol version mismatch in StatusResp: expected {}, got {} (seq: {}, my_seq: {})",
                                PROTOCOL_VERSION, version, resp_seq, my_seq
                            );
                            return Err(anyhow::anyhow!(
                                "Protocol version mismatch in StatusResp: expected {}, got {}",
                                PROTOCOL_VERSION,
                                version
                            ));
                        }
                        if resp_seq != my_seq {
                            eprintln!(
                                "[PlayerNode] Sequence mismatch in StatusResp: expected {}, got {}",
                                my_seq, resp_seq
                            );
                            return Err(anyhow::anyhow!(
                                "Sequence mismatch in StatusResp: expected {}, got {}",
                                my_seq,
                                resp_seq
                            ));
                        }
                        // This should be unreachable due to the guard above, but handle it safely
                        return Err(anyhow::anyhow!("Unexpected StatusResp state"));
                    }
                    other => {
                        eprintln!(
                            "[PlayerNode] Unexpected message type when expecting StatusResp: {:?} (my_seq: {})",
                            other, my_seq
                        );
                        return Err(anyhow::anyhow!(
                            "Expected StatusResp, got unexpected message type (closing session)"
                        ));
                    }
                };
                my_seq += 1;
                let res_common = match res_domain {
                    DomainGuessResult::Hit => GuessResult::Hit,
                    DomainGuessResult::Miss => GuessResult::Miss,
                    DomainGuessResult::Sink(name) => {
                        let static_name = ship_name_static(&name)
                            .ok_or_else(|| anyhow::anyhow!("unknown ship"))?;
                        GuessResult::Sink(static_name)
                    }
                };
                self.engine
                    .record_guess(r, c, res_common)
                    .map_err(|e| anyhow::anyhow!(e))?;
                self.player.handle_guess_result((r, c), res_common);
                my_turn = false;
            } else {
                // Receive opponent guess and respond
                let msg = self.transport.recv().await?;
                match msg {
                    Message::Guess {
                        version,
                        seq: msg_seq,
                        x,
                        y,
                    } => {
                        // Validate version
                        if version != PROTOCOL_VERSION {
                            eprintln!(
                                "[PlayerNode] Protocol version mismatch in Guess: expected {}, got {} (seq: {})",
                                PROTOCOL_VERSION, version, msg_seq
                            );
                            return Err(anyhow::anyhow!(
                                "Protocol version mismatch in Guess: expected {}, got {}",
                                PROTOCOL_VERSION,
                                version
                            ));
                        }
                        
                        // Validate sequence number
                        if msg_seq != expected_recv_seq {
                            eprintln!(
                                "[PlayerNode] Out-of-order Guess: expected seq {}, got {}",
                                expected_recv_seq, msg_seq
                            );
                            return Err(anyhow::anyhow!(
                                "Out-of-order Guess: expected seq {}, got {}",
                                expected_recv_seq,
                                msg_seq
                            ));
                        }
                        
                        let res_common = self
                            .engine
                            .opponent_guess(x as usize, y as usize)
                            .map_err(|e| anyhow::anyhow!(e))?;
                        self.player
                            .handle_opponent_guess((x as usize, y as usize), res_common);
                        let res_domain = DomainGuessResult::from(res_common);
                        self.transport
                            .send(Message::StatusResp {
                                version: PROTOCOL_VERSION,
                                seq: msg_seq,
                                res: res_domain,
                            })
                            .await?;
                        my_turn = true;
                        expected_recv_seq += 1;
                    }
                    other => {
                        eprintln!(
                            "[PlayerNode] Unexpected message type when expecting Guess: {:?} (expected_seq: {})",
                            other, expected_recv_seq
                        );
                        return Err(anyhow::anyhow!(
                            "Expected Guess message, got unexpected message type (closing session)"
                        ))
                    }
                }
            }

            if !matches!(self.engine.status(), GameStatus::InProgress) {
                break;
            }
        }
        Ok(())
    }

    /// Current status of the underlying game engine.
    pub fn status(&self) -> GameStatus {
        self.engine.status()
    }

    /// Total number of guesses this player has made.
    pub fn guess_count(&self) -> usize {
        self.engine.guess_hits().count_ones() + self.engine.guess_misses().count_ones()
    }
}
