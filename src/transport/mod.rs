use crate::protocol::Message;

#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, msg: Message) -> anyhow::Result<()>;
    async fn recv(&mut self) -> anyhow::Result<Message>;
}

#[cfg(feature = "std")]
pub mod tcp;
#[cfg(feature = "std")]
pub mod in_memory;
