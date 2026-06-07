# Docs And Test Alignment

Completed documentation and guidance cleanup for the post-refactor adapter architecture.

## Completion Summary

- README commands now distinguish default tests, all-feature tests/checks, no-default-feature checks, and clippy.
- `overview.md` now includes a compact module-boundary diagram, feature verification checklist, and a note that the active root crate no longer has `src/interface`.
- Added targeted AI-agent guidance for tracked tests and planning documents.
- Re-checked tracked test filenames for stale `interface` naming; no tracked test paths matched, so no test rename was needed.
- The active root crate has no tracked `examples/` directory, so no examples were promoted as supported public API examples.

## Verification

- Documentation-only implementation, so full tests were skipped per repository guidance.
- During planning, these command shapes were verified to compile:
  - `cargo check --no-default-features`
  - `cargo check --all-features`
  - `cargo test --all-features --no-run`
  - `cargo test --no-default-features --no-run`
