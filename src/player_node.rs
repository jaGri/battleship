#![cfg(feature = "std")]

extern crate alloc;

use alloc::boxed::Box;
use rand::rngs::SmallRng;

use crate::{
    domain::GuessResult as DomainGuessResult,
    game::GameStatus,
    player::Player,
    transport::Transport,
    GameEngine, protocol::Message, common::GuessResult,
};

pub struct PlayerNode {
    player: Box<dyn Player>,
    engine: GameEngine,
    transport: Box<dyn Transport>,
}

impl PlayerNode {
    pub fn new(player: Box<dyn Player>, engine: GameEngine, transport: Box<dyn Transport>) -> Self {
        Self { player, engine, transport }
    }

    pub async fn run(&mut self, rng: &mut SmallRng, first_move: bool) -> anyhow::Result<()> {
        let mut my_turn = first_move;
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
                    .send(Message::Guess { x: r as u8, y: c as u8 })
                    .await?;
                let reply = self.transport.recv().await?;
                let res_domain = match reply {
                    Message::StatusResp(res) => res,
                    _ => return Err(anyhow::anyhow!("unexpected reply")),
                };
                let res_common = match res_domain {
                    DomainGuessResult::Hit => GuessResult::Hit,
                    DomainGuessResult::Miss => GuessResult::Miss,
                    DomainGuessResult::Sink => GuessResult::Hit,
                };
                self.engine
                    .record_guess(r, c, res_common)
                    .map_err(|e| anyhow::anyhow!(e))?;
                self.player.handle_guess_result((r, c), res_common);
                my_turn = false;
            } else {
                // Receive opponent guess and respond
                let msg = self.transport.recv().await?;
                if let Message::Guess { x, y } = msg {
                    let res_common = self
                        .engine
                        .opponent_guess(x as usize, y as usize)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    self.player
                        .handle_opponent_guess((x as usize, y as usize), res_common);
                    let res_domain = DomainGuessResult::from(res_common);
                    self.transport
                        .send(Message::StatusResp(res_domain))
                        .await?;
                } else {
                    continue;
                }
                my_turn = true;
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

