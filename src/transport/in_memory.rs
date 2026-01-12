#![cfg(feature = "std")]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::task::yield_now;
use tokio::time::{sleep, Duration};

use crate::protocol::Message;
use crate::transport::Transport;

/// State shared between paired transports.
struct SharedState {
    queue: VecDeque<Message>,
    closed: bool,
}

impl SharedState {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            closed: false,
        }
    }
}

pub struct InMemoryTransport {
    recv_state: Arc<Mutex<SharedState>>,
    send_state: Arc<Mutex<SharedState>>,
    shutdown: Arc<AtomicBool>,
}

impl InMemoryTransport {
    pub fn pair() -> (Self, Self) {
        let state1 = Arc::new(Mutex::new(SharedState::new()));
        let state2 = Arc::new(Mutex::new(SharedState::new()));
        
        (
            Self {
                recv_state: state1.clone(),
                send_state: state2.clone(),
                shutdown: Arc::new(AtomicBool::new(false)),
            },
            Self {
                recv_state: state2,
                send_state: state1,
                shutdown: Arc::new(AtomicBool::new(false)),
            },
        )
    }

    /// Request graceful shutdown of the transport.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        
        // Mark send channel as closed so peer can detect it
        if let Ok(mut state) = self.send_state.lock() {
            state.closed = true;
        }
    }

    /// Check if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Check if the peer has closed the channel.
    fn is_peer_closed(&self) -> bool {
        if let Ok(state) = self.recv_state.lock() {
            state.closed && state.queue.is_empty()
        } else {
            true
        }
    }
}

#[async_trait::async_trait]
impl Transport for InMemoryTransport {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }

        let mut state = self.send_state.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire send lock"))?;
        
        if state.closed {
            return Err(anyhow::anyhow!("Channel closed by peer"));
        }
        
        state.queue.push_back(msg);
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }

        // Poll with exponential backoff to reduce CPU usage
        let mut backoff_ms = 1;
        let max_backoff_ms = 100;
        
        loop {
            // Check for early channel closure detection
            if self.is_peer_closed() {
                return Err(anyhow::anyhow!("Channel closed by peer"));
            }

            // Try to receive a message
            let msg_opt = {
                let mut state = self.recv_state.lock()
                    .map_err(|_| anyhow::anyhow!("Failed to acquire receive lock"))?;
                
                // Double-check closed state under lock
                if state.closed && state.queue.is_empty() {
                    return Err(anyhow::anyhow!("Channel closed by peer"));
                }
                
                state.queue.pop_front()
            };

            if let Some(msg) = msg_opt {
                return Ok(msg);
            }

            // Check if peer still exists using weak reference count
            // If only 1 strong reference remains, the peer has dropped
            if Arc::strong_count(&self.recv_state) == 1 {
                return Err(anyhow::anyhow!("Channel closed: peer dropped"));
            }

            // Yield or sleep with backoff
            if backoff_ms < 10 {
                yield_now().await;
                backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
            } else {
                sleep(Duration::from_millis(backoff_ms as u64)).await;
                backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
            }
        }
    }
}

impl Drop for InMemoryTransport {
    fn drop(&mut self) {
        // Mark our send channel as closed when dropping
        if let Ok(mut state) = self.send_state.lock() {
            state.closed = true;
        }
    }
}
