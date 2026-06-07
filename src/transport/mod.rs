use crate::protocol::WireMessage;

/// Nonblocking transport endpoint used by app runners.
pub trait TransportEndpoint {
    type Error;

    fn poll(&mut self) -> Result<Option<WireMessage>, Self::Error>;
    fn send(&mut self, msg: &WireMessage) -> Result<(), Self::Error>;
    fn is_connected(&self) -> bool;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, msg: WireMessage) -> anyhow::Result<()>;
    async fn recv(&mut self) -> anyhow::Result<WireMessage>;
}

#[cfg(feature = "std")]
pub mod tcp;
#[cfg(feature = "std")]
pub mod in_memory;
#[cfg(feature = "std")]
pub mod heartbeat;
