//! BLE transport adapter boundary for embedded builds.

use crate::protocol::WireMessage;
use crate::transport::TransportEndpoint;

/// Maximum frame size accepted by the default BLE wire helpers.
pub const MAX_BLE_FRAME_BYTES: usize = 256;

/// Fixed-capacity BLE frame buffer for simulator and embedded adapters.
pub type BleFrame = heapless::Vec<u8, MAX_BLE_FRAME_BYTES>;

/// Errors returned by BLE frame helpers and the disconnected boundary endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleTransportError {
    Disconnected,
    FrameTooLarge,
    Encode,
    Decode,
}

/// BLE endpoint boundary for platform-specific implementations.
///
/// This type is intentionally disconnected until a runner supplies platform BLE
/// I/O, pairing, link encryption, entropy, and time sources around it.
#[derive(Debug, Default)]
pub struct BleTransport {
    connected: bool,
}

impl BleTransport {
    /// Create a disconnected BLE adapter boundary.
    pub fn new() -> Self {
        Self::disconnected()
    }

    /// Create a disconnected BLE adapter boundary.
    pub fn disconnected() -> Self {
        Self { connected: false }
    }
}

impl TransportEndpoint for BleTransport {
    type Error = BleTransportError;

    fn poll(&mut self) -> Result<Option<WireMessage>, Self::Error> {
        Ok(None)
    }

    fn send(&mut self, _msg: &WireMessage) -> Result<(), Self::Error> {
        Err(BleTransportError::Disconnected)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Encode a wire message into a bounded BLE frame.
pub fn encode_wire_message(msg: &WireMessage) -> Result<BleFrame, BleTransportError> {
    let mut bytes = [0u8; MAX_BLE_FRAME_BYTES];
    let encoded = postcard::to_slice(msg, &mut bytes).map_err(|err| match err {
        postcard::Error::SerializeBufferFull => BleTransportError::FrameTooLarge,
        _ => BleTransportError::Encode,
    })?;
    BleFrame::from_slice(encoded).map_err(|_| BleTransportError::FrameTooLarge)
}

/// Decode a wire message from a bounded BLE frame.
pub fn decode_wire_message(frame: &[u8]) -> Result<WireMessage, BleTransportError> {
    if frame.len() > MAX_BLE_FRAME_BYTES {
        return Err(BleTransportError::FrameTooLarge);
    }
    postcard::from_bytes(frame).map_err(|_| BleTransportError::Decode)
}
