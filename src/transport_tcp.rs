#[cfg(feature = "std")]
use tokio::net::{TcpStream, ToSocketAddrs};
#[cfg(feature = "std")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::transport::Transport;
use crate::protocol::Message;

#[cfg(feature = "std")]
pub struct TcpTransport {
    stream: TcpStream,
}

#[cfg(feature = "std")]
impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn connect<A: ToSocketAddrs>(addr: A) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self::new(stream))
    }
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        let data = bincode::serialize(&msg)?;
        let len = (data.len() as u32).to_be_bytes();
        self.stream.write_all(&len).await?;
        self.stream.write_all(&data).await?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf).await?;
        let msg = bincode::deserialize(&buf)?;
        Ok(msg)
    }
}
