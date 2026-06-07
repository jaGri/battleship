//! WebSocket transport adapter boundary.

use crate::protocol::WireMessage;
use crate::transport::TransportEndpoint;

/// Nonblocking WebSocket endpoint shell.
pub struct WebSocketTransport {
    url: String,
    connected: bool,
}

impl WebSocketTransport {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connected: false,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl TransportEndpoint for WebSocketTransport {
    type Error = std::io::Error;

    fn poll(&mut self) -> Result<Option<WireMessage>, Self::Error> {
        Ok(None)
    }

    fn send(&mut self, _msg: &WireMessage) -> Result<(), Self::Error> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
