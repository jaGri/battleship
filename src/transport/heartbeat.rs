#![cfg(feature = "std")]

use tokio::time::{interval, Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::protocol::{Message, PROTOCOL_VERSION};
use crate::transport::Transport;

/// Transport wrapper that adds active heartbeat monitoring and idle connection detection.
///
/// HeartbeatTransport wraps any Transport implementation and adds:
/// - Periodic heartbeat message sending (configurable interval)
/// - Automatic heartbeat response (echo back)
/// - Idle connection timeout detection
/// - Transparent heartbeat filtering (heartbeats not returned to caller)
///
/// Can be disabled for transports that don't need heartbeat monitoring (e.g., InMemoryTransport).
pub struct HeartbeatTransport<T: Transport> {
    inner: T,
    heartbeat_interval: Duration,
    idle_timeout: Duration,
    last_activity: Instant,
    enabled: bool,
    shutdown: Arc<AtomicBool>,
}

impl<T: Transport> HeartbeatTransport<T> {
    /// Create a new HeartbeatTransport with active heartbeat monitoring.
    ///
    /// # Arguments
    /// * `inner` - The underlying transport to wrap
    /// * `heartbeat_interval` - How often to send heartbeat messages when idle
    /// * `idle_timeout` - Maximum idle time before considering connection dead
    pub fn new(inner: T, heartbeat_interval: Duration, idle_timeout: Duration) -> Self {
        Self {
            inner,
            heartbeat_interval,
            idle_timeout,
            last_activity: Instant::now(),
            enabled: true,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a HeartbeatTransport with heartbeat monitoring disabled.
    ///
    /// This is useful for transports that don't need connection health monitoring
    /// (e.g., InMemoryTransport). The wrapper adds minimal overhead in disabled mode.
    pub fn disabled(inner: T) -> Self {
        Self {
            inner,
            heartbeat_interval: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(45),
            last_activity: Instant::now(),
            enabled: false,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Update the last activity timestamp.
    fn mark_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if the connection has exceeded the idle timeout.
    fn is_idle_timeout(&self) -> bool {
        self.last_activity.elapsed() > self.idle_timeout
    }

    /// Request graceful shutdown of the transport.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl<T: Transport> Transport for HeartbeatTransport<T> {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }

        if self.enabled && self.is_idle_timeout() {
            return Err(anyhow::anyhow!("Connection idle timeout exceeded"));
        }

        let result = self.inner.send(msg).await;
        if result.is_ok() {
            self.mark_activity();
        }
        result
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        if !self.enabled {
            // Fast path: no heartbeat logic, just delegate to inner transport
            return self.inner.recv().await;
        }

        if self.shutdown.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }

        let mut heartbeat_timer = interval(self.heartbeat_interval);
        heartbeat_timer.tick().await; // First tick completes immediately, skip it

        loop {
            tokio::select! {
                // Receive message from inner transport
                msg_result = self.inner.recv() => {
                    match msg_result {
                        Ok(Message::Heartbeat { version }) => {
                            // Validate protocol version
                            if version != PROTOCOL_VERSION {
                                eprintln!(
                                    "[HeartbeatTransport] Heartbeat version mismatch: expected {}, got {}",
                                    PROTOCOL_VERSION, version
                                );
                                return Err(anyhow::anyhow!(
                                    "Heartbeat version mismatch: expected {}, got {}",
                                    PROTOCOL_VERSION, version
                                ));
                            }

                            // Mark activity and echo heartbeat back
                            self.mark_activity();
                            if let Err(e) = self.inner.send(Message::Heartbeat {
                                version: PROTOCOL_VERSION
                            }).await {
                                eprintln!("[HeartbeatTransport] Failed to echo heartbeat: {}", e);
                                return Err(e);
                            }

                            // Continue loop to get next message (filter out heartbeat)
                            continue;
                        }
                        Ok(msg) => {
                            // Regular game message - mark activity and return
                            self.mark_activity();
                            return Ok(msg);
                        }
                        Err(e) => {
                            // Propagate error from inner transport
                            return Err(e);
                        }
                    }
                }

                // Periodic heartbeat sending
                _ = heartbeat_timer.tick() => {
                    // Check for idle timeout
                    if self.is_idle_timeout() {
                        eprintln!("[HeartbeatTransport] Idle timeout exceeded ({:?})", self.idle_timeout);
                        return Err(anyhow::anyhow!(
                            "Connection idle timeout exceeded ({:?})",
                            self.idle_timeout
                        ));
                    }

                    // Send heartbeat
                    if let Err(e) = self.inner.send(Message::Heartbeat {
                        version: PROTOCOL_VERSION
                    }).await {
                        eprintln!("[HeartbeatTransport] Failed to send heartbeat: {}", e);
                        return Err(e);
                    }

                    self.mark_activity();
                }
            }
        }
    }
}
