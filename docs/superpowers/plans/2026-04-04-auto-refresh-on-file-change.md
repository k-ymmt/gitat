# Auto-Refresh on File Change Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically detect file changes in the target repository and refresh Uncommitted Changes display using OS-native filesystem watching.

**Architecture:** Migrate the main event loop from synchronous `crossterm::event::poll` to `tokio::select!`, adding `notify-debouncer-full` for filesystem watching with 500ms debounce. `gitat-ui` and `gitat-core` remain synchronous — only the `gitat` main crate adopts async.

**Tech Stack:** tokio 1 (current_thread), notify 8, notify-debouncer-full 0.7, crossterm 0.28

---

### Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml:9-18` (workspace dependencies)
- Modify: `crates/gitat/Cargo.toml:6-11` (main crate dependencies)

- [ ] **Step 1: Add workspace dependencies**

In `Cargo.toml`, add `tokio`, `notify`, and `notify-debouncer-full` to `[workspace.dependencies]`:

```toml
[workspace.dependencies]
gitat-core = { path = "crates/gitat-core" }
gitat-ui = { path = "crates/gitat-ui" }
thiserror = "2"
anyhow = "1"
ratatui = "0.30.0"
crossterm = "0.28"
similar = "2"
insta = "1.47.2"
unicode-width = "0.2"
tokio = { version = "1", features = ["rt", "macros", "sync"] }
notify = "8"
notify-debouncer-full = "0.7"
```

- [ ] **Step 2: Add main crate dependencies**

In `crates/gitat/Cargo.toml`, add the new dependencies:

```toml
[package]
name = "gitat"
version.workspace = true
edition.workspace = true

[dependencies]
gitat-core = { workspace = true }
gitat-ui = { workspace = true }
anyhow = { workspace = true }
crossterm = { workspace = true }
ratatui = { workspace = true }
tokio = { workspace = true }
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
```

- [ ] **Step 3: Verify dependencies resolve**

Run: `cargo check -p gitat 2>&1 | tail -5`
Expected: compilation succeeds (warnings are OK)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/gitat/Cargo.toml
git commit -m "build: add tokio, notify, notify-debouncer-full dependencies"
```

---

### Task 2: Add `refresh_status_and_log()` to App

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:128-138` (add method after `refresh()`)
- Test: `crates/gitat-ui/src/app.rs` (inline tests module)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `crates/gitat-ui/src/app.rs`:

```rust
#[test]
fn test_refresh_status_and_log_updates_status_and_log() {
    use gitat_core::runner::MockRunner;

    let runner = MockRunner::new()
        .with_response("status --porcelain=v1", " M src/main.rs\n")
        .with_response(
            "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e",
            "abc123\x1fabc\x1f\x1fHEAD -> main\x1fAuthor\x1f2026-04-04\x1fInitial commit\x1e",
        );

    let mut app = App::new();
    app.refresh_status_and_log(&runner);

    assert_eq!(app.status.len(), 1);
    assert_eq!(app.log_entries.len(), 1);
    // branches should remain unchanged (not refreshed)
    assert!(app.branches.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_refresh_status_and_log_updates_status_and_log 2>&1 | tail -10`
Expected: FAIL — `refresh_status_and_log` method does not exist

- [ ] **Step 3: Write minimal implementation**

Add the `refresh_status_and_log` method to `impl App` in `crates/gitat-ui/src/app.rs`, right after the existing `refresh()` method (after line 138):

```rust
pub fn refresh_status_and_log(&mut self, runner: &dyn CommandRunner) {
    if let Ok(status) = gitat_core::status::get_status(runner) {
        self.status = status;
    }
    if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
        self.log_entries = log;
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_refresh_status_and_log_updates_status_and_log 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 5: Run all existing tests to verify no regressions**

Run: `cargo test -p gitat-ui 2>&1 | tail -10`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat: add App::refresh_status_and_log() for lightweight fs-triggered refresh"
```

---

### Task 3: Migrate Main Event Loop to Async with Filesystem Watcher

**Files:**
- Modify: `crates/gitat/src/main.rs` (full rewrite of `main()` and `run_app()`)

- [ ] **Step 1: Add imports**

Replace the imports at the top of `crates/gitat/src/main.rs` with:

```rust
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};
use tokio::sync::mpsc;

use gitat_core::runner::ProcessRunner;
use gitat_ui::app::{App, Mode, Tab};
use gitat_ui::event::handle_key;
use gitat_ui::theme::Theme;
use gitat_ui::util::centered_rect;
use gitat_ui::views;
use gitat_ui::views::commit::render_commit_popup;
```

- [ ] **Step 2: Rewrite `main()` to async**

Replace the `fn main()` function with:

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let repo_path = std::env::current_dir().context("failed to get current directory")?;
    let runner = ProcessRunner::new(repo_path.clone());

    let mut terminal = ratatui::init();
    let mut app = App::new();
    app.refresh(&runner);
    app.log_list_state.select(Some(0));
    gitat_ui::event::load_log_preview(&mut app, &runner);

    let result = run_app(&mut terminal, &mut app, &runner, &repo_path).await;

    ratatui::restore();
    result
}
```

- [ ] **Step 3: Rewrite `run_app()` to async with `tokio::select!`**

Replace the `fn run_app()` function with:

```rust
async fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    runner: &ProcessRunner,
    repo_path: &Path,
) -> Result<()> {
    // Spawn blocking task for crossterm key input
    let (key_tx, mut key_rx) = mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || loop {
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Ok(ev) = event::read() {
                if key_tx.send(ev).is_err() {
                    break;
                }
            }
        }
    });

    // Setup filesystem watcher
    let (mut fs_rx, _debouncer) = setup_watcher(repo_path)?;

    loop {
        app.clear_expired_status_message();
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // tabs
                    Constraint::Min(0),     // main content
                    Constraint::Length(1),  // status bar
                ])
                .split(f.area());

            // Tab bar
            let tab_titles: Vec<Line> = [Tab::Log, Tab::Branches, Tab::Stash]
                .iter()
                .map(|t| {
                    let style = if *t == app.tab {
                        Theme::tab_active()
                    } else {
                        Theme::tab_inactive()
                    };
                    Line::from(Span::styled(t.title(), style))
                })
                .collect();

            let tabs = Tabs::new(tab_titles)
                .select(match app.tab {
                    Tab::Log => 0,
                    Tab::Branches => 1,
                    Tab::Stash => 2,
                })
                .highlight_style(Theme::tab_active());
            f.render_widget(tabs, chunks[0]);

            // Main content
            views::render_tab(f, app, chunks[1]);

            // Status bar
            let status_text = if let Some(ref msg) = app.status_message {
                msg.clone()
            } else {
                "j/k: move  h/l: panel  Tab: switch  s: stage  c: commit  p: push  ?: help  q: quit".to_string()
            };
            let status_bar = Paragraph::new(status_text).style(Theme::status_bar());
            f.render_widget(status_bar, chunks[2]);

            // Conflict editor overlay
            if matches!(app.mode, Mode::Conflict { .. })
                && let (Some(file), Some(state)) =
                    (&app.conflict_file, &mut app.conflict_state)
            {
                let editor =
                    gitat_ui::widgets::conflict_editor::ConflictEditor::new(file);
                f.render_stateful_widget(editor, chunks[1], state);
            }

            // Popups
            if matches!(app.mode, Mode::Commit { .. }) {
                render_commit_popup(f, app);
            }

            if matches!(app.mode, Mode::Help) {
                render_help_popup(f);
            }
        })?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            Some(ev) = key_rx.recv() => {
                match ev {
                    Event::Key(key) => handle_key(app, key, runner),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
            Some(()) = fs_rx.recv() => {
                app.refresh_status_and_log(runner);
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Add `setup_watcher()` function**

Add this function after `run_app()`, before `render_help_popup()`:

```rust
fn setup_watcher(
    repo_path: &Path,
) -> Result<(mpsc::Receiver<()>, notify_debouncer_full::Debouncer<notify::RecommendedWatcher, notify_debouncer_full::NoCache>)> {
    let (tx, rx) = mpsc::channel::<()>(1);

    let mut debouncer = new_debouncer(
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

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p gitat 2>&1 | tail -10`
Expected: compilation succeeds

- [ ] **Step 6: Run all tests**

Run: `cargo test 2>&1 | tail -15`
Expected: all tests pass

- [ ] **Step 7: Manual verification**

Run the app in a git repository:
```bash
cargo run
```

In another terminal, modify a file in the same repository:
```bash
echo "test" >> /tmp/test-file.txt
```

Expected: the "Uncommitted Changes" line in the Log tab updates within ~1 second without pressing any key.

- [ ] **Step 8: Commit**

```bash
git add crates/gitat/src/main.rs
git commit -m "feat: async event loop with filesystem watcher for auto-refresh

Migrate main event loop from synchronous crossterm::event::poll to
tokio::select!, adding notify-debouncer-full for OS-native filesystem
watching with 500ms debounce. Detects changes in working tree and
.git/ directory, automatically refreshing status and log data."
```

---

### Task 4: Final Verification

- [ ] **Step 1: Run clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -20`
Expected: no errors (warnings about unused variables are acceptable)

- [ ] **Step 2: Run full test suite**

Run: `cargo test 2>&1 | tail -15`
Expected: all tests pass

- [ ] **Step 3: Fix any clippy warnings or test failures**

If there are issues, fix them and re-run.

- [ ] **Step 4: Commit fixes if any**

```bash
git add -A
git commit -m "fix: address clippy warnings from async migration"
```
