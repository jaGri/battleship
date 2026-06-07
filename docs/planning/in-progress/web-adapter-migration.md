# Web Adapter Migration

Replace the remaining legacy web `Interface` path with focused web adapters.

## Current State

- `src/interface` remains only for websocket/web compatibility.
- `WebInterface` still implements the old mixed `Interface` trait.
- Some web board conversion logic is placeholder-level.

## Next Questions

- What is the web runtime shape: browser client, server session, or both?
- Should web input produce `UiEvent` directly or protocol messages?
- Which existing web tests should migrate first?

## Candidate Work

- Create `WebInput` for web-originated UI events.
- Create `WebRenderer` or web view serializer from `ScreenView`.
- Move websocket session behavior toward `AppEvent` and `AppCommand`.
- Replace placeholder board conversion with real board/view conversion.
- Remove `src/interface` once web no longer depends on the legacy trait.
