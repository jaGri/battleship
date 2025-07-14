#![cfg(feature = "std")]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::task::yield_now;

use crate::protocol::Message;
use crate::transport::Transport;

pub struct InMemoryTransport {
    recv_queue: Arc<Mutex<VecDeque<Message>>>,
    send_queue: Arc<Mutex<VecDeque<Message>>>,
}

impl InMemoryTransport {
    pub fn pair() -> (Self, Self) {
        let q1 = Arc::new(Mutex::new(VecDeque::new()));
        let q2 = Arc::new(Mutex::new(VecDeque::new()));
        (
            Self {
                recv_queue: q1.clone(),
                send_queue: q2.clone(),
            },
            Self {
                recv_queue: q2,
                send_queue: q1,
            },
        )
    }
}

#[async_trait::async_trait]
impl Transport for InMemoryTransport {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        let mut queue = self.send_queue.lock().unwrap();
        queue.push_back(msg);
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        loop {
            if let Some(msg) = {
                let mut queue = self.recv_queue.lock().unwrap();
                queue.pop_front()
            } {
                return Ok(msg);
            }
            if Arc::strong_count(&self.recv_queue) == 1 {
                return Err(anyhow::anyhow!("Channel closed"));
            }
            yield_now().await;
        }
    }
}

