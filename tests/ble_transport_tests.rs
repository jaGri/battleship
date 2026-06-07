#![cfg(feature = "ble")]

use battleship::protocol::{WireMessage, PROTOCOL_VERSION};
use battleship::transport::ble::{
    decode_wire_message, encode_wire_message, BleTransport, BleTransportError, MAX_BLE_FRAME_BYTES,
};
use battleship::transport::TransportEndpoint;

#[test]
fn ble_frame_helpers_round_trip_wire_message() {
    let msg = WireMessage::Heartbeat {
        version: PROTOCOL_VERSION,
    };

    let frame = encode_wire_message(&msg).unwrap();
    let restored = decode_wire_message(&frame).unwrap();

    assert_eq!(restored, msg);
}

#[test]
fn ble_frame_helpers_reject_oversized_frames() {
    let oversized = [0u8; MAX_BLE_FRAME_BYTES + 1];

    let err = decode_wire_message(&oversized).unwrap_err();

    assert_eq!(err, BleTransportError::FrameTooLarge);
}

#[test]
fn ble_frame_helpers_reject_malformed_frames() {
    let malformed = [0xFF, 0x00, 0x01];

    let err = decode_wire_message(&malformed).unwrap_err();

    assert_eq!(err, BleTransportError::Decode);
}

#[test]
fn default_ble_transport_is_disconnected_boundary() {
    let mut transport = BleTransport::new();
    let msg = WireMessage::Heartbeat {
        version: PROTOCOL_VERSION,
    };

    assert!(!transport.is_connected());
    assert_eq!(transport.poll().unwrap(), None);
    assert_eq!(
        transport.send(&msg).unwrap_err(),
        BleTransportError::Disconnected
    );
}
