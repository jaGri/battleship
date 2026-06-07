# Persistence App Integration

Wire persistence into the app loop instead of keeping it as a separate subsystem.

## Current State

- Persistence types, validation, SQLite storage, embedded storage shape, sync concepts, and tests exist.
- `AppCommand::Save` and `AppCommand::ClearSave` exist but carry no data.
- `AppEvent` has no explicit save-loaded event.

## Next Questions

- What is the app-level saved-game shape?
- Should persistence store `GameEngine::state()` directly or a richer app snapshot?
- How should resume/new-game conflicts be presented to renderers?

## Candidate Work

- Define an app snapshot type.
- Add `AppEvent::SaveLoaded(...)` or equivalent.
- Make `AppCommand::Save` carry the snapshot to persist.
- Add startup resume flow.
- Clear active save on completed games.
- Add tests with in-memory persistence before SQLite-specific tests.
