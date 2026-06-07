# Remote Play And Transport Runner

Connect the app state machine to remote opponents over transports.

## Completion Summary

- Added `TransportCommandRunner<T>` as the reusable runner bridge for
  `AppCommand::Send`, inbound `WireMessage` polling, and transport connection
  transitions.
- Kept remote game decisions in `BattleshipApp`; transports still only move
  protocol messages.
- Added an in-memory remote-play integration test that drives two apps through
  setup and a guess/status exchange using paired `InMemoryTransport` endpoints.
- Updated `README.md`, `overview.md`, and test guidance to describe the bridge
  and keep concrete TCP/WebSocket/BLE runner UX as future adapter work.

## Verification

- `cargo fmt`
- `cargo test --test app_state_machine_tests --all-features`
- `cargo test --test adapter_architecture_tests --all-features`
- `cargo test --test remote_play_tests --all-features`
- `cargo check --no-default-features`
- `cargo check --all-features`
- `cargo test --all-features`
- `cargo clippy --all-features -- -D warnings`

## Current State

- `WireMessage` exists and transports move it.
- `BattleshipApp` processes remote handshake, ready, guess, status, resume, and
  private sync messages through `AppEvent::Transport(WireMessage)`.
- `RemoteAgent` exists for transport-observed remote actions.
- `TransportCommandRunner<T>` connects app command queues to `TransportEndpoint`
  implementations.

## Resolved Questions

- Setup uses handshake, handshake acknowledgement, and ready messages.
- Turn exchange uses guess messages and status responses.
- Resume/sync uses resume requests, resume acknowledgements, and hidden-information-safe private sync payloads.
- Remote match state lives in `BattleshipApp`; `RemoteAgent` remains an adapter
  role, and the transport runner only bridges commands and messages.
- Outgoing messages are represented as `AppCommand::Send`; retries and
  reconnect UI are represented through app connection states and runner-emitted
  connection events.

## Completed Work

- Added `RemoteAgent`.
- Implemented protocol-to-app-event mapping in `BattleshipApp`.
- Implemented the app-command-to-transport runner bridge.
- Added in-memory remote-play integration coverage.
- Defined reducer behavior for disconnect, reconnect, duplicate message,
  out-of-order message, invalid message, and version mismatch cases.
