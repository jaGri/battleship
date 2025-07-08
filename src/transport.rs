use crate::protocol::Message;
use alloc::boxed::Box;

#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()>;
    async fn recv(&mut self) -> anyhow::Result<Message>;
}