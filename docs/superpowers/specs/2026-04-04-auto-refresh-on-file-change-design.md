# Auto-Refresh on File Change Design

## Summary

Add automatic detection and reflection of file changes in the target repository to the Log screen's "Uncommitted Changes" display. Currently, the UI only updates on manual refresh (`r` key) or after explicit git operations. This design introduces OS-native filesystem watching via the `notify` crate and migrates the main event loop to `tokio` async runtime.

## Requirements

- Detect file changes in the target repository (including `.git/` directory) using OS-native filesystem APIs
- Automatically update `status` and `log` data when changes are detected
- Debounce rapid changes (500ms) to avoid excessive `git status` / `git log` calls
- Migrate the main event loop from synchronous `crossterm::event::poll` to `tokio::select!`

## Design

### 1. Dependency Additions

Add to workspace `Cargo.toml`:

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 (features: `rt`, `macros`, `sync`) | Async runtime |
| `notify-debouncer-full` | 0.7 | Filesystem watching with debounce |
| `notify` | 8 | Types re-exported by notify-debouncer-full |

**Crate-level dependencies:**
- `gitat` (main crate): `tokio`, `notify-debouncer-full`, `notify`
- `gitat-ui` and `gitat-core`: No changes (remain synchronous)

### 2. Main Event Loop Async Migration

Convert `main()` from synchronous loop to `tokio` async runtime.

**Runtime flavor:** `current_thread` — single-threaded runtime is sufficient for a TUI application.

**Key input handling:** Use `tokio::task::spawn_blocking` to run `crossterm::event::poll` + `crossterm::event::read` in the blocking thread pool. Send events to the main loop via `tokio::sync::mpsc::unbounded_channel`.

**Event multiplexing:** Use `tokio::select!` to await both key input and filesystem change notifications.

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // terminal setup...

    let (key_tx, mut key_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::task::spawn_blocking(move || {
        loop {
            if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
                if let Ok(ev) = crossterm::event::read() {
                    if key_tx.send(ev).is_err() { break; }
                }
            }
        }
    });

    let (mut fs_rx, _debouncer) = setup_watcher(&repo_path)?;

    loop {
        app.clear_expired_status_message();
        terminal.draw(|f| { /* render */ })?;
        if app.should_quit { break; }

        tokio::select! {
            Some(ev) = key_rx.recv() => {
                match ev {
                    Event::Key(key) => handle_key(&mut app, key, &runner),
                    Event::Resize(_, _) => {},
                    _ => {}
                }
            }
            Some(_) = fs_rx.recv() => {
                app.refresh_status_and_log(&runner);
            }
        }
    }

    // terminal cleanup...
}
```

### 3. Filesystem Watcher Setup

```rust
fn setup_watcher(
    repo_path: &Path,
) -> Result<(
    tokio::sync::mpsc::Receiver<()>,
    notify_debouncer_full::Debouncer<RecommendedWatcher, FileIdMap>,
)> {
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);

    let debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            if result.is_ok() {
                let _ = tx.try_send(());
            }
        },
    )?;

    debouncer.watch(repo_path, RecursiveMode::Recursive)?;

    Ok((rx, debouncer))
}
```

**Design decisions:**
- **Channel capacity 1 + `try_send`**: The notification is a signal ("something changed"), not a queue of events. If the buffer is full, the event is dropped — it will be handled in the next `select!` cycle.
- **Debounce 500ms**: Aggregates rapid changes (editor saves, build tools) into a single notification.
- **`RecursiveMode::Recursive`**: Watches the entire repository including `.git/` directory.
- **Debouncer ownership**: Returned to the caller to keep the watcher alive. Dropping the debouncer stops watching.
- **Event content ignored**: Regardless of which file changed, a full `status` + `log` refresh is performed.

### 4. App Refresh Method Extension

Add a lightweight refresh method to `App` that only updates `status` and `log`:

```rust
impl App {
    // Existing: full data refresh (manual refresh, after operations)
    pub fn refresh(&mut self, runner: &dyn CommandRunner) { /* unchanged */ }

    // New: status + log only (triggered by filesystem changes)
    pub fn refresh_status_and_log(&mut self, runner: &dyn CommandRunner) {
        self.status = gitat_core::status::get_status(runner).unwrap_or_default();
        self.log_entries = gitat_core::log::get_log(runner, 100, None).unwrap_or_default();
    }
}
```

- `refresh()` remains unchanged — used by `r` key and post-operation refreshes
- `refresh_status_and_log()` skips `branches` for lighter execution
- Error handling matches existing pattern (`unwrap_or_default()`)

### 5. `.git/` Directory Monitoring

Including `.git/` in the watch scope enables detection of:
- `.git/index` changes: external `git add` / `git reset`
- `.git/refs/heads/` changes: external `git commit` / `git branch`
- `.git/HEAD` changes: external `git checkout`

**Noise sources:**
- `.git/objects/`, `.git/logs/`, `.git/COMMIT_EDITMSG` — internal git operations

**Mitigation:**
- 500ms debounce aggregates most noise into a single event
- gitat's own operations (stage/commit) also trigger filesystem events, causing a redundant refresh. Cost is one additional `git status` + `git log` execution — negligible.
- No special path filtering is applied. Debounce is sufficient for now.

### 6. `.gitignore` Handling

`notify` does not respect `.gitignore` — changes in `node_modules/`, `target/`, etc. will trigger events. This is acceptable because:
- Debounce aggregates these into a single event
- `git status` itself correctly handles `.gitignore`
- Additional filtering (e.g., `ignore` crate) can be added later if needed

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `tokio`, `notify-debouncer-full`, `notify` to workspace dependencies |
| `crates/gitat/Cargo.toml` | Add dependencies to main crate |
| `crates/gitat/src/main.rs` | Async event loop, watcher setup, spawn_blocking for key input |
| `crates/gitat-ui/src/app.rs` | Add `refresh_status_and_log()` method |

## Out of Scope

- Path-based filtering for `.gitignore` or `.git/` subdirectories
- Suppression of redundant refresh after gitat's own operations
- Watching for branch changes (branches are only refreshed on manual `r` key or operations)
- Async migration of `gitat-ui` or `gitat-core` crates
