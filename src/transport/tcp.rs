#![cfg(feature = "std")]

use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::transport::Transport;
use crate::protocol::{Message, PROTOCOL_VERSION};

/// Default timeout for network operations (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum message size (10 MB) to prevent excessive memory allocation.
const MAX_MESSAGE_SIZE: u32 = 10_000_000;

/// Default heartbeat interval (10 seconds).
const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

/// Default idle timeout before considering connection dead (45 seconds).
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(45);

#[cfg(feature = "std")]
pub struct TcpTransport {
    stream: TcpStream,
    timeout_duration: Duration,
    max_message_size: u32,
    #[allow(dead_code)] // Reserved for future heartbeat implementation
    heartbeat_interval: Duration,
    idle_timeout: Duration,
    last_activity: Instant,
    shutdown: Arc<AtomicBool>,
}

#[cfg(feature = "std")]
impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { 
            stream,
            timeout_duration: DEFAULT_TIMEOUT,
            max_message_size: MAX_MESSAGE_SIZE,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            last_activity: Instant::now(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_timeout(stream: TcpStream, timeout_duration: Duration) -> Self {
        Self {
            stream,
            timeout_duration,
            max_message_size: MAX_MESSAGE_SIZE,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            last_activity: Instant::now(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_config(
        stream: TcpStream, 
        timeout_duration: Duration,
        max_message_size: u32,
        heartbeat_interval: Duration,
        idle_timeout: Duration,
    ) -> Self {
        Self {
            stream,
            timeout_duration,
            max_message_size,
            heartbeat_interval,
            idle_timeout,
            last_activity: Instant::now(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn connect<A: ToSocketAddrs>(addr: A) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self::new(stream))
    }

    /// Request graceful shutdown of the transport.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Check if the connection has been idle for too long.
    pub fn is_idle_timeout(&self) -> bool {
        self.last_activity.elapsed() > self.idle_timeout
    }

    /// Send a heartbeat message.
    pub async fn send_heartbeat(&mut self) -> anyhow::Result<()> {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }
        
        let heartbeat = Message::Heartbeat { version: PROTOCOL_VERSION };
        self.send(heartbeat).await
    }

    /// Update the last activity timestamp.
    fn mark_activity(&mut self) {
        self.last_activity = Instant::now();
    }
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }
        
        if self.is_idle_timeout() {
            return Err(anyhow::anyhow!("Connection idle timeout exceeded"));
        }
        
        let send_op = async {
            let data = bincode::serialize(&msg)
                .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
            
            // Check if serialized message exceeds max size
            if data.len() as u32 > self.max_message_size {
                return Err(anyhow::anyhow!(
                    "Message too large: {} bytes (max: {})", 
                    data.len(), 
                    self.max_message_size
                ));
            }
            
            let len = (data.len() as u32).to_be_bytes();
            self.stream.write_all(&len).await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::BrokenPipe 
                        || e.kind() == std::io::ErrorKind::ConnectionReset {
                        anyhow::anyhow!("Connection closed by peer")
                    } else {
                        anyhow::anyhow!("Write error: {}", e)
                    }
                })?;
            
            self.stream.write_all(&data).await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::BrokenPipe 
                        || e.kind() == std::io::ErrorKind::ConnectionReset {
                        anyhow::anyhow!("Connection closed by peer")
                    } else {
                        anyhow::anyhow!("Write error: {}", e)
                    }
                })?;
            
            anyhow::Ok(())
        };
        
        let result = timeout(self.timeout_duration, send_op)
            .await
            .map_err(|_| anyhow::anyhow!("Send timeout after {:?}", self.timeout_duration))?;
        
        if result.is_ok() {
            self.mark_activity();
        }
        
        result
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        if self.is_shutdown() {
            return Err(anyhow::anyhow!("Transport is shut down"));
        }
        
        if self.is_idle_timeout() {
            return Err(anyhow::anyhow!("Connection idle timeout exceeded"));
        }
        
        let recv_op = async {
            let mut len_buf = [0u8; 4];
            self.stream.read_exact(&mut len_buf).await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        anyhow::anyhow!("Connection closed by peer")
                    } else if e.kind() == std::io::ErrorKind::ConnectionReset {
                        anyhow::anyhow!("Connection reset by peer")
                    } else {
                        anyhow::anyhow!("Read error: {}", e)
                    }
                })?;
            
            let len = u32::from_be_bytes(len_buf);
            
            // Bounded read length check to prevent excessive memory allocation
            if len > self.max_message_size {
                return Err(anyhow::anyhow!(
                    "Message too large: {} bytes (max: {})", 
                    len, 
                    self.max_message_size
                ));
            }
            
            if len == 0 {
                return Err(anyhow::anyhow!("Invalid message length: 0"));
            }
            
            let mut buf = vec![0u8; len as usize];
            self.stream.read_exact(&mut buf).await
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        anyhow::anyhow!("Connection closed by peer")
                    } else if e.kind() == std::io::ErrorKind::ConnectionReset {
                        anyhow::anyhow!("Connection reset by peer")
                    } else {
                        anyhow::anyhow!("Read error: {}", e)
                    }
                })?;
            
            let msg = bincode::deserialize(&buf)
                .map_err(|e| anyhow::anyhow!("Deserialization error: {}", e))?;
            
            anyhow::Ok(msg)
        };
        
        let result = timeout(self.timeout_duration, recv_op)
            .await
            .map_err(|_| anyhow::anyhow!("Receive timeout after {:?}", self.timeout_duration))?;
        
        if result.is_ok() {
            self.mark_activity();
        }
        
        result
    }
}
