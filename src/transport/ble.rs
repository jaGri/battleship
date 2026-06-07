//! BLE transport adapter boundary for embedded builds.

use crate::protocol::WireMessage;
use crate::transport::TransportEndpoint;

/// Placeholder BLE endpoint for platform-specific implementations.
#[derive(Debug, Default)]
pub struct BleTransport {
    connected: bool,
}

impl BleTransport {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl TransportEndpoint for BleTransport {
    type Error = core::convert::Infallible;

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
