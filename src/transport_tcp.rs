#[cfg(feature = "std")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(feature = "std")]
use tokio::net::TcpStream;
#[cfg(feature = "std")]

use crate::protocol::{Message, SyncPayload};
use crate::transport::Transport;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

#[cfg(feature = "std")]
pub struct TcpTransport {
    stream: TcpStream,
}

#[cfg(feature = "std")]
impl TcpTransport {
    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub async fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        let data = encode_message(&msg);
        let len = data.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await?;
        self.stream.write_all(&data).await?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        self.stream.read_exact(&mut buf).await?;
        decode_message(&buf)
    }
}

#[cfg(feature = "std")]
fn encode_message(msg: &Message) -> Vec<u8> {
    let mut v = Vec::new();
    match msg {
        Message::Guess { x, y } => {
            v.push(0);
            v.push(*x);
            v.push(*y);
        }
        Message::StatusReq => v.push(1),
        Message::StatusResp(res) => {
            v.push(2);
            v.push(match res {
                crate::common::GuessResult::Hit => 0,
                crate::common::GuessResult::Miss => 1,
                crate::common::GuessResult::Sink(_) => 2,
            });
        }
        Message::Sync(_) => v.push(3),
        Message::Ack => v.push(4),
    }
    v
}

#[cfg(feature = "std")]
fn decode_message(buf: &[u8]) -> anyhow::Result<Message> {
    Ok(match buf[0] {
        0 => Message::Guess { x: buf[1], y: buf[2] },
        1 => Message::StatusReq,
        2 => {
            let res = match buf[1] {
                0 => crate::common::GuessResult::Hit,
                1 => crate::common::GuessResult::Miss,
                _ => crate::common::GuessResult::Sink(""),
            };
            Message::StatusResp(res)
        }
        3 => Message::Sync(SyncPayload),
        4 => Message::Ack,
        _ => return Err(anyhow::anyhow!("invalid message")),
    })
}
