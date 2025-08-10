#![cfg(all(feature = "std", feature = "ble"))]

use crate::protocol::Message;
use crate::transport::Transport;

/// Maximum payload size for a single BLE packet.
const BLE_MTU: usize = 20;

/// Placeholder trait representing the underlying BLE connection.
#[async_trait::async_trait]
pub trait BleConnection: Send + Sync {
    /// Send a single chunk of bytes over the BLE link.
    async fn write(&mut self, data: &[u8]) -> anyhow::Result<()>;

    /// Receive a chunk of bytes from the BLE link.
    async fn read(&mut self) -> anyhow::Result<Vec<u8>>;
}

/// Transport implementation backed by a BLE connection.
pub struct BleTransport<C: BleConnection> {
    conn: C,
    recv_buf: Vec<u8>,
}

impl<C: BleConnection> BleTransport<C> {
    /// Create a new transport from the given BLE connection.
    pub fn new(conn: C) -> Self {
        Self {
            conn,
            recv_buf: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl<C: BleConnection> Transport for BleTransport<C> {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()> {
        let data = bincode::serialize(&msg)?;
        let mut frame = (data.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(&data);
        for chunk in frame.chunks(BLE_MTU) {
            self.conn.write(chunk).await?; // placeholder BLE write
        }
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<Message> {
        loop {
            if self.recv_buf.len() >= 4 {
                let len = u32::from_be_bytes(self.recv_buf[0..4].try_into().unwrap()) as usize;
                if self.recv_buf.len() >= 4 + len {
                    let data = self.recv_buf[4..4 + len].to_vec();
                    self.recv_buf.drain(..4 + len);
                    let msg = bincode::deserialize(&data)?;
                    return Ok(msg);
                }
            }
            let chunk = self.conn.read().await?; // placeholder BLE read
            self.recv_buf.extend_from_slice(&chunk);
        }
    }
}
