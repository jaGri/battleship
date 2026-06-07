# Docs And Test Alignment

Keep docs, examples, and tests aligned with the post-refactor architecture.

## Current State

- README and examples have been partially updated for the adapter architecture.
- Active tests pass.
- Some test names still mention interface even though they now test agents.

## Next Questions

- Which docs are intended for users versus maintainers?
- Should test filenames be renamed now or after more app work?
- Which examples should be treated as supported public API examples?

## Candidate Work

- Rename AI interface tests to AI agent tests.
- Update stale command names in README test examples.
- Add architecture diagrams for current single-crate module boundaries.
- Add a compatibility note for the remaining web legacy module.
- Add a verification checklist for feature combinations.
