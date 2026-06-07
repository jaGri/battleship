# Embedded And Security Hardening

Replace placeholder embedded/security behavior with production-ready implementations or clear test doubles.

## Current State

- BLE transport/security modules exist.
- Persistence security types exist.
- Some embedded entropy, timestamp, hashing, and HMAC logic is explicitly placeholder-level.

## Next Questions

- Which embedded target is first-class for verification: ESP-IDF, simulator, or both?
- What crypto guarantees are required for local saves versus remote anti-cheat?
- Which code should be production implementation versus test-only mock?

## Candidate Work

- Replace placeholder hash/HMAC helpers with real implementations.
- Use real platform entropy for embedded game IDs and auth tokens.
- Use real monotonic or wall-clock time for embedded timestamps.
- Add simulator-friendly tests for embedded persistence behavior.
- Document security boundaries honestly.
