# gitat

A terminal-based Git client (TUI) written in Rust using ratatui.

## Build & Test Commands

- Build: `cargo build`
- Test all: `cargo test`
- Test single crate: `cargo test -p gitat-core` or `cargo test -p gitat-ui`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Snapshot review: `cargo insta review`
- Run: `cargo run`

## Architecture

Three-crate workspace:

- **gitat-core** — Git command execution and output parsing. No UI dependencies. Uses `CommandRunner` trait for testability (real `ProcessRunner` + `MockRunner` for tests).
- **gitat-ui** — TUI layer using ratatui. Contains `App` state, event handlers (`event/`), view renderers (`views/`), and custom widgets (`widgets/`).
- **gitat** — Binary entry point. Async event loop with tokio, filesystem watcher for auto-refresh.

## Code Conventions

- Rust Edition 2024, toolchain 1.94.1 (pinned in mise.toml)
- Use `thiserror` for library error types in gitat-core
- Use `anyhow` for application-level errors in the binary crate
- No `unwrap()` in library code — use proper error handling
- Mode-based event dispatch: each `Mode` variant has its own handler in `event/`
- Views are pure rendering functions that take `&App` and `&mut Frame`

## Testing

- Snapshot testing with `insta::assert_debug_snapshot!()` for widget rendering and git output parsing
- Snapshots stored in `src/snapshots/` within each crate
- Use `TestBackend` from ratatui for headless widget rendering tests
- Use `MockRunner` (implements `CommandRunner`) for git command mocking
- IMPORTANT: Run `cargo insta review` to interactively accept/reject snapshot changes

## Key Types

- `App` (gitat-ui/src/app.rs) — Central state struct
- `Mode` enum — Normal, Commit, Conflict, Search, Help, CommitDetail, UncommittedDetail
- `Tab` enum — Log, Branches, Stash
- `Panel` enum — Left (file list), Right (diff viewer)
- `UnifiedDiffState` — Diff viewer widget state with scrolling
- `ConflictEditorState` — Merge conflict resolution editor state
