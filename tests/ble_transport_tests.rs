#![cfg(feature = "ble")]

use async_trait::async_trait;
use battleship::protocol::Message;
use battleship::transport::ble::{BleConnection, BleTransport};
use battleship::transport::Transport;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::task::yield_now;

struct MockBle {
    recv_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
    send_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl MockBle {
    fn pair() -> (Self, Self) {
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

#[async_trait]
impl BleConnection for MockBle {
    async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let mut q = self.send_queue.lock().unwrap();
        q.push_back(data.to_vec());
        Ok(())
    }

    async fn read(&mut self) -> anyhow::Result<Vec<u8>> {
        loop {
            if let Some(chunk) = {
                let mut q = self.recv_queue.lock().unwrap();
                q.pop_front()
            } {
                return Ok(chunk);
            }
            if Arc::strong_count(&self.recv_queue) == 1 {
                return Err(anyhow::anyhow!("Channel closed"));
            }
            yield_now().await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ble_round_trip() -> anyhow::Result<()> {
    let (dev1, dev2) = MockBle::pair();
    let mut t1 = BleTransport::new(dev1);
    let mut t2 = BleTransport::new(dev2);

    let msg = Message::Ack;
    t1.send(msg.clone()).await?;
    let recv = t2.recv().await?;
    assert!(matches!(recv, Message::Ack));
    Ok(())
}
