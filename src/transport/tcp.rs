#![cfg(feature = "std")]

use super::Transport;
use crate::protocol::Message;

pub struct TcpTransport;

#[async_trait::async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, _msg: Message) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn recv(&mut self) -> anyhow::Result<Message> {
        unimplemented!()
    }
}
