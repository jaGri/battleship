# Embedded And Security Hardening

## Completion Summary

Completed the verifiable hardening path for the current root crate.

- File-backed active saves now use an authenticated BLAKE3 envelope with magic,
  version, payload length, and MAC fields.
- Corrupted, tampered, unsupported-version, and legacy raw-bincode save files are
  rejected before deserialization.
- BLE now exposes an explicit disconnected adapter boundary with bounded
  postcard frame helpers for `WireMessage`.
- Documentation now states that local saves are integrity-protected but not
  encrypted, and that BLE link security, entropy, timestamps, remote auth, and
  anti-cheat remain platform-runner responsibilities.

## Verification

- Added persistence tests for round trips, missing/cleared saves, tampered
  payloads, tampered headers, unsupported versions, and legacy raw bincode.
- Added BLE tests for frame round trips, oversized/malformed frames, and the
  disconnected default boundary.

## Resolved Questions

- First-class verification target: simulator-friendly root-crate behavior.
- Local save guarantee: integrity/authentication, not encryption.
- Remote anti-cheat/auth and embedded platform security: adapter-owned future
  work, outside the core/app/transport message boundary.
