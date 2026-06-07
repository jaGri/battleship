# Web Adapter Migration

Completed the first tracked-root web adapter slice.

## Current State

- The root crate exposes a feature-gated `web` module with `WebInput`, `WebRenderer`, and owned web view models.
- Browser-originated actions normalize to `UiEvent`; websocket/network frames remain runner-owned `WireMessage` traffic.
- `WebRenderer` converts passive `ScreenView` values into owned serializable web data, including real board, guess-board, ship, status, connection, and event state.
- The tracked root crate has no `src/interface` module. The untracked nested `battleship/` reference folder was not edited.

## Verification

- Added focused web adapter tests covering input normalization, target validation, game board conversion, non-game views, and app-flow rendering.
- Verified with `cargo fmt`, `cargo check --features web`, `cargo test --features web`, `cargo check --no-default-features`, `cargo test --all-features`, and `cargo clippy --all-features -- -D warnings`.
- On this Windows environment, clippy required the rustup GNU toolchain with a pinned `RUSTC` and temporary target directory to avoid mixed Chocolatey/rustup artifacts.

## Remaining Work

- Build an actual browser/server runner that serializes `WebScreenView` values and bridges websocket traffic through `AppEvent::Transport` and `AppCommand::Send`.
- Expand websocket transport behavior independently from the web view adapter.
