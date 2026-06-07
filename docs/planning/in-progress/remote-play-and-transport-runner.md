# Remote Play And Transport Runner

Connect the app state machine to remote opponents over transports.

## Current State

- `WireMessage` exists and transports move it.
- `BattleshipApp` accepts `AppEvent::Transport(WireMessage)` but does not process it yet.
- There is no dedicated `RemoteAgent`.

## Next Questions

- Which protocol messages are required for setup, turn exchange, resume, sync, and game over?
- Should remote state live in a `RemoteAgent`, a transport runner, or both?
- How should app commands represent outgoing transport messages and retry/reconnect behavior?

## Candidate Work

- Add `RemoteAgent` or equivalent remote-decision adapter.
- Implement protocol-to-app-event mapping.
- Implement app-command-to-transport runner loop.
- Add in-memory remote-play integration tests before TCP/WebSocket-specific tests.
- Define behavior for disconnect, reconnect, timeout, duplicate message, and invalid message.
