# Persistence App Integration

Completed persistence app-loop integration so active saves are runner-owned but
driven by `BattleshipApp` commands.

## Completed Work

- Added `AppCommand::LoadActiveSave` for runner-owned active save loading.
- Changed the "Resume Game" menu item to request an active save load.
- Kept restore data flowing through `AppEvent::Loaded(Some(SavedGame))`.
- Changed missing-save handling to clear stale active saves, render the main
  menu, and show a "No saved game." notice.
- Added `FileSaveStore` for std runners, using `bincode` snapshots in a file.
- Wired the CLI runner to save, load, and clear `battleship.sav` in the current
  working directory.
- Clear active saves when a game reaches `GameOver`.

## Verification

- App reducer tests cover resume load requests, missing saves, saved-game
  restoration, and game-over clear-save behavior.
- Persistence tests cover in-memory saves plus file-backed save round trips,
  missing-file loads, and clearing missing files.
- Final verification should include `cargo fmt`, `cargo check --no-default-features`,
  `cargo test --all-features`, and `cargo clippy --all-features -- -D warnings`.

