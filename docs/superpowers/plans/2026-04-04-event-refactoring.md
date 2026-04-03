# Event Module Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `event.rs` (730 lines) into focused submodules, extract duplicated `is_staged` logic, and add missing error notification.

**Architecture:** Extract `StatusEntry::is_staged()` method to `gitat-core`. Split `event.rs` into `event/mod.rs` (dispatch + small handlers), `event/normal.rs`, `event/staging.rs`, `event/commit_detail.rs`. Add status message for diff reload failure.

**Tech Stack:** Rust, ratatui, gitat-core, gitat-ui

---

## File Structure

| File | Responsibility |
|---|---|
| Modify: `crates/gitat-core/src/status.rs` | Add `is_staged()` method to `StatusEntry` |
| Delete: `crates/gitat-ui/src/event.rs` | Replaced by `event/` directory |
| Create: `crates/gitat-ui/src/event/mod.rs` | `handle_key` dispatch + `handle_commit`, `handle_help`, `handle_search`, `handle_conflict` |
| Create: `crates/gitat-ui/src/event/normal.rs` | `handle_normal` + `list_len` |
| Create: `crates/gitat-ui/src/event/staging.rs` | `stage_or_unstage`, `stage_or_unstage_hunk`, `load_diff_for_selected` |
| Create: `crates/gitat-ui/src/event/commit_detail.rs` | `handle_commit_detail`, `enter_commit_detail`, `load_commit_detail_diff` |
| Modify: `TODO.md` | Mark completed items |

---

### Task 1: Add `is_staged()` to `StatusEntry`

**Files:**
- Modify: `crates/gitat-core/src/status.rs`

- [ ] **Step 1: Write test for `is_staged()`**

Add to the existing `#[cfg(test)] mod tests` in `crates/gitat-core/src/status.rs`:

```rust
#[test]
fn test_is_staged_modified_in_index() {
    let entry = StatusEntry {
        path: "file.rs".to_string(),
        index_status: FileStatus::Modified,
        worktree_status: FileStatus::Unmodified,
    };
    assert!(entry.is_staged());
}

#[test]
fn test_is_staged_added_in_index() {
    let entry = StatusEntry {
        path: "file.rs".to_string(),
        index_status: FileStatus::Added,
        worktree_status: FileStatus::Unmodified,
    };
    assert!(entry.is_staged());
}

#[test]
fn test_is_staged_unmodified_not_staged() {
    let entry = StatusEntry {
        path: "file.rs".to_string(),
        index_status: FileStatus::Unmodified,
        worktree_status: FileStatus::Modified,
    };
    assert!(!entry.is_staged());
}

#[test]
fn test_is_staged_untracked_not_staged() {
    let entry = StatusEntry {
        path: "file.rs".to_string(),
        index_status: FileStatus::Untracked,
        worktree_status: FileStatus::Untracked,
    };
    assert!(!entry.is_staged());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-core -- test_is_staged`
Expected: 4 failures — `is_staged` method does not exist.

- [ ] **Step 3: Implement `is_staged()` on `StatusEntry`**

In `crates/gitat-core/src/status.rs`, add an `impl` block after the `StatusEntry` struct definition (after line 20):

```rust
impl StatusEntry {
    pub fn is_staged(&self) -> bool {
        !matches!(
            self.index_status,
            FileStatus::Unmodified | FileStatus::Untracked
        )
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-core -- test_is_staged`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-core/src/status.rs
git commit -m "refactor: add is_staged() method to StatusEntry"
```

---

### Task 2: Create `event/mod.rs`

**Files:**
- Create: `crates/gitat-ui/src/event/mod.rs`

This is a mechanical move. The original `event.rs` stays intact until all 4 files are ready; then we swap in Task 5.

- [ ] **Step 1: Create the `event/` directory**

```bash
mkdir -p crates/gitat-ui/src/event
```

- [ ] **Step 2: Write `event/mod.rs`**

Create `crates/gitat-ui/src/event/mod.rs` with the following content. This contains `handle_key`, the small mode handlers (`handle_commit`, `handle_help`, `handle_search`, `handle_conflict`), and submodule declarations:

```rust
mod commit_detail;
mod normal;
mod staging;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode};
use gitat_core::runner::CommandRunner;

pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match app.mode {
        Mode::Normal => normal::handle_normal(app, key, runner),
        Mode::Commit { .. } => handle_commit(app, key, runner),
        Mode::Help => handle_help(app, key),
        Mode::Search { .. } => handle_search(app, key),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => commit_detail::handle_commit_detail(app, key, runner),
    }
}

fn handle_commit(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let message = match &app.mode {
                Mode::Commit { message } => message.clone(),
                _ => return,
            };
            if message.is_empty() {
                app.set_status_message("Commit message cannot be empty");
                return;
            }
            match gitat_core::commit::commit(runner, &message) {
                Ok(()) => {
                    app.mode = Mode::Normal;
                    app.set_status_message("Committed successfully");
                    app.refresh(runner);
                }
                Err(e) => {
                    app.set_status_message(format!("Commit failed: {e}"));
                    app.mode = Mode::Normal;
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::Commit { message } = &mut app.mode {
                message.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Commit { message } = &mut app.mode {
                message.push(c);
            }
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            if let Mode::Search { query } = &mut app.mode {
                query.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Search { query } = &mut app.mode {
                query.push(c);
            }
        }
        _ => {}
    }
}

fn handle_conflict(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    let (state, file) = match (&mut app.conflict_state, &app.conflict_file) {
        (Some(state), Some(file)) => (state, file),
        _ => {
            app.mode = Mode::Normal;
            return;
        }
    };

    match key.code {
        KeyCode::Char('o') => state.use_ours(file),
        KeyCode::Char('t') => state.use_theirs(file),
        KeyCode::Char('n') => state.next_conflict(),
        KeyCode::Char('N') => state.prev_conflict(),
        KeyCode::Char('w') => {
            let content = state.result_content();
            let path = file.path.clone();
            if let Err(e) = gitat_core::conflict::resolve_file(runner, &path, &content) {
                app.set_status_message(format!("Resolve failed: {e}"));
            } else {
                app.set_status_message(format!("Resolved: {path}"));
                app.refresh(runner);
            }
            app.conflict_state = None;
            app.conflict_file = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.conflict_state = None;
            app.conflict_file = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use gitat_core::runner::MockRunner;

    use crate::app::{Panel, Tab};

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_quit() {
        let mut app = App::new();
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('q')), &runner);
        assert!(app.should_quit);
    }

    #[test]
    fn test_tab_switch() {
        let mut app = App::new();
        let runner = MockRunner::new();
        assert_eq!(app.tab, Tab::Status);
        handle_key(&mut app, mock_key(KeyCode::Tab), &runner);
        assert_eq!(app.tab, Tab::Branches);
    }

    #[test]
    fn test_panel_switch() {
        let mut app = App::new();
        let runner = MockRunner::new();
        assert_eq!(app.panel, Panel::Left);
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn test_enter_commit_mode() {
        let mut app = App::new();
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
    }

    #[test]
    fn test_escape_commit_mode() {
        let mut app = App::new();
        app.mode = Mode::Commit {
            message: String::new(),
        };
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }
}
```

- [ ] **Step 3: Verify the file was created**

```bash
wc -l crates/gitat-ui/src/event/mod.rs
```

Expected: approximately 180 lines.

---

### Task 3: Create `event/normal.rs`, `event/staging.rs`, `event/commit_detail.rs`

**Files:**
- Create: `crates/gitat-ui/src/event/normal.rs`
- Create: `crates/gitat-ui/src/event/staging.rs`
- Create: `crates/gitat-ui/src/event/commit_detail.rs`

- [ ] **Step 1: Write `event/normal.rs`**

Create `crates/gitat-ui/src/event/normal.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel, Tab};
use gitat_core::runner::CommandRunner;

pub(super) fn handle_normal(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.tab = app.tab.next();
        }
        KeyCode::BackTab => {
            app.tab = app.tab.prev();
        }
        // Diff navigation keys (uppercase, right panel only) — must be checked before lowercase h/l
        KeyCode::Char('n') if app.panel == Panel::Right => {
            app.diff_state.next_hunk();
        }
        KeyCode::Char('N') if app.panel == Panel::Right => {
            app.diff_state.prev_hunk();
        }
        KeyCode::Char('J') if app.panel == Panel::Right => {
            app.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') if app.panel == Panel::Right => {
            app.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') if app.panel == Panel::Right => {
            app.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') if app.panel == Panel::Right => {
            app.diff_state.scroll_right(4);
        }
        KeyCode::Char('l') => {
            app.panel = Panel::Right;
        }
        KeyCode::Char('h') => {
            app.panel = Panel::Left;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let len = list_len(app);
            if len > 0 {
                let state = app.current_list_state_mut();
                let i = match state.selected() {
                    Some(i) => (i + 1).min(len - 1),
                    None => 0,
                };
                state.select(Some(i));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let state = app.current_list_state_mut();
            if let Some(i) = state.selected() {
                let next = if i == 0 { 0 } else { i - 1 };
                state.select(Some(next));
            }
        }
        KeyCode::Char('s') => {
            if app.tab == Tab::Status {
                match app.panel {
                    Panel::Left => super::staging::stage_or_unstage(app, runner),
                    Panel::Right => super::staging::stage_or_unstage_hunk(app, runner),
                }
            }
        }
        KeyCode::Char('c') => {
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('p') => {
            match gitat_core::branch::current_branch(runner) {
                Ok(branch) => {
                    if let Err(e) = gitat_core::remote::push(runner, "origin", &branch) {
                        app.set_status_message(format!("Push failed: {e}"));
                    } else {
                        app.set_status_message(format!("Pushed to origin/{branch}"));
                    }
                }
                Err(e) => app.set_status_message(format!("Push failed: {e}")),
            }
        }
        KeyCode::Char('P') => {
            match gitat_core::branch::current_branch(runner) {
                Ok(branch) => {
                    if let Err(e) = gitat_core::remote::pull(runner, "origin", &branch) {
                        app.set_status_message(format!("Pull failed: {e}"));
                    } else {
                        app.set_status_message(format!("Pulled from origin/{branch}"));
                        app.refresh(runner);
                    }
                }
                Err(e) => app.set_status_message(format!("Pull failed: {e}")),
            }
        }
        KeyCode::Char('b') => {
            app.set_status_message("Branch creation: not yet implemented");
        }
        KeyCode::Char('d') => {
            if app.tab == Tab::Branches {
                delete_selected_branch(app, runner);
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search {
                query: String::new(),
            };
        }
        KeyCode::Char('r') => {
            app.refresh(runner);
            app.set_status_message("Refreshed");
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        KeyCode::Enter => {
            if app.tab == Tab::Log {
                super::commit_detail::enter_commit_detail(app, runner);
            } else {
                super::staging::load_diff_for_selected(app, runner);
            }
        }
        _ => {}
    }
}

fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::Status => app.status.len(),
        Tab::Branches => app.branches.len(),
        Tab::Log => app.log_entries.len(),
        Tab::Stash => 0,
    }
}

fn delete_selected_branch(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.branches_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let branch = match app.branches.get(idx) {
        Some(b) => b.clone(),
        None => return,
    };
    if branch.is_current {
        app.set_status_message("Cannot delete current branch");
        return;
    }
    match gitat_core::branch::delete_branch(runner, &branch.name) {
        Ok(()) => {
            app.refresh(runner);
            app.set_status_message(format!("Deleted branch '{}'", branch.name));
        }
        Err(e) => {
            app.set_status_message(format!("Delete branch failed: {e}"));
        }
    }
}
```

- [ ] **Step 2: Write `event/staging.rs`**

Create `crates/gitat-ui/src/event/staging.rs`. This uses the new `entry.is_staged()` method. Also includes the diff reload failure notification fix (spec item 3):

```rust
use crate::app::{App, Tab};
use crate::widgets::side_by_side_diff::SideBySideDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn stage_or_unstage(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let result = if entry.is_staged() {
        gitat_core::stage::unstage_file(runner, &entry.path)
    } else {
        gitat_core::stage::stage_file(runner, &entry.path)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage failed: {e}"));
        }
    }
}

pub(super) fn stage_or_unstage_hunk(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let diff_files = match &app.current_diff {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };

    let diff_file = match diff_files
        .iter()
        .find(|f| f.new_path == entry.path || f.old_path == entry.path)
    {
        Some(f) => f.clone(),
        None => return,
    };

    let hunk_index = app.diff_state.current_hunk;
    let is_staged = entry.is_staged();

    let result = if is_staged {
        gitat_core::stage::unstage_hunk(runner, &diff_file, hunk_index)
    } else {
        gitat_core::stage::stage_hunk(runner, &diff_file, hunk_index)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
            // After staging, show remaining unstaged diff; after unstaging, show remaining staged diff
            match gitat_core::diff::get_diff_for_file(runner, &entry.path, !is_staged) {
                Ok(diff) => {
                    if diff.is_empty() || diff.iter().all(|f| f.hunks.is_empty()) {
                        app.current_diff = None;
                        app.diff_state = SideBySideDiffState::new();
                    } else {
                        // Clamp current_hunk
                        let total_hunks: usize = diff.iter().map(|f| f.hunks.len()).sum();
                        if app.diff_state.current_hunk >= total_hunks {
                            app.diff_state.current_hunk = total_hunks.saturating_sub(1);
                        }
                        app.current_diff = Some(diff);
                    }
                }
                Err(e) => {
                    app.set_status_message(format!("Failed to reload diff: {e}"));
                    app.current_diff = None;
                    app.diff_state = SideBySideDiffState::new();
                }
            }
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage hunk failed: {e}"));
        }
    }
}

pub(super) fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if app.tab != Tab::Status {
        return;
    }
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    match gitat_core::diff::get_diff_for_file(runner, &entry.path, entry.is_staged()) {
        Ok(diff) => {
            app.current_diff = Some(diff);
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Panel, Tab};
    use crate::event::handle_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_s_in_right_panel_calls_stage_hunk() {
        let mut app = App::new();
        app.tab = Tab::Status;
        app.panel = Panel::Right;

        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Unmodified,
            worktree_status: gitat_core::status::FileStatus::Modified,
        }];
        app.status_list_state.select(Some(0));

        app.current_diff = Some(vec![gitat_core::diff::DiffFile {
            old_path: "src/main.rs".to_string(),
            new_path: "src/main.rs".to_string(),
            hunks: vec![gitat_core::diff::DiffHunk {
                old_start: 1,
                old_count: 2,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Context,
                        content: "line1".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Added,
                        content: "new_line".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Context,
                        content: "line2".to_string(),
                        old_line_no: Some(2),
                        new_line_no: Some(3),
                    },
                ],
            }],
        }]);
        app.diff_state.current_hunk = 0;

        let log_key =
            "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e";
        let runner = MockRunner::new()
            .with_response("apply --cached", "")
            .with_response("status --porcelain=v1", "")
            .with_response("branch -v --no-color", "")
            .with_response(log_key, "")
            .with_response("diff -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);

        assert!(
            app.status_message.is_none()
                || !app.status_message.as_ref().unwrap().contains("failed")
        );
    }

    #[test]
    fn test_s_in_left_panel_still_stages_file() {
        let mut app = App::new();
        app.tab = Tab::Status;
        app.panel = Panel::Left;
        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Unmodified,
            worktree_status: gitat_core::status::FileStatus::Modified,
        }];
        app.status_list_state.select(Some(0));

        let runner = MockRunner::new()
            .with_response("add -- src/main.rs", "")
            .with_response("status --porcelain=v1", "")
            .with_response("branch -v --no-color", "")
            .with_response(
                "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e",
                "",
            );

        handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);
        assert!(app.status_message.is_none());
    }
}
```

- [ ] **Step 3: Write `event/commit_detail.rs`**

Create `crates/gitat-ui/src/event/commit_detail.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::side_by_side_diff::SideBySideDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let commit = match app.log_entries.get(idx) {
        Some(c) => c.clone(),
        None => return,
    };

    let files = match gitat_core::commit_detail::get_commit_files(runner, &commit.hash) {
        Ok(f) => f,
        Err(e) => {
            app.set_status_message(format!("Failed to load commit files: {e}"));
            return;
        }
    };

    app.commit_detail_commit = Some(commit);
    app.commit_detail_files = files;
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_panel = Panel::Left;
    app.commit_detail_diff_state = SideBySideDiffState::new();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    app.mode = Mode::CommitDetail;
}

fn load_commit_detail_diff(app: &mut App, runner: &dyn CommandRunner) {
    let commit = match &app.commit_detail_commit {
        Some(c) => c,
        None => return,
    };
    let idx = match app.commit_detail_file_state.selected() {
        Some(i) => i,
        None => return,
    };
    let file_entry = match app.commit_detail_files.get(idx) {
        Some(f) => f,
        None => return,
    };

    let parent = commit.parent_hashes.first().map(|s| s.as_str());
    match gitat_core::commit_detail::get_commit_file_diff(
        runner,
        &commit.hash,
        parent,
        &file_entry.path,
    ) {
        Ok(diff) => {
            app.commit_detail_diff = Some(diff);
            app.commit_detail_diff_state = SideBySideDiffState::new();
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_commit_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.commit_detail_commit = None;
            app.commit_detail_files.clear();
            app.commit_detail_file_state = ratatui::widgets::ListState::default();
            app.commit_detail_diff = None;
            app.commit_detail_diff_state = SideBySideDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.commit_detail_panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.commit_detail_panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.commit_detail_panel == Panel::Left {
                let len = app.commit_detail_files.len();
                if len > 0 {
                    let i = match app.commit_detail_file_state.selected() {
                        Some(i) => (i + 1).min(len - 1),
                        None => 0,
                    };
                    app.commit_detail_file_state.select(Some(i));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail_diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.commit_detail_panel == Panel::Left {
                if let Some(i) = app.commit_detail_file_state.selected() {
                    let next = if i == 0 { 0 } else { i - 1 };
                    app.commit_detail_file_state.select(Some(next));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail_diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('J') => {
            app.commit_detail_diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.commit_detail_diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.commit_detail_diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.commit_detail_diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.commit_detail_diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.commit_detail_diff_state.prev_hunk();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Mode, Panel, Tab};
    use crate::event::handle_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_enter_log_tab_enters_commit_detail_mode() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.log_entries = vec![gitat_core::log::CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            author: "Test".to_string(),
            date: "2026-04-04".to_string(),
            message: "test commit".to_string(),
            refs: vec![],
            parent_hashes: vec!["parent1".to_string()],
        }];
        app.log_list_state.select(Some(0));

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
    }

    #[test]
    fn test_esc_from_commit_detail_returns_to_normal() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        app.commit_detail_commit = Some(gitat_core::log::CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            author: "Test".to_string(),
            date: "2026-04-04".to_string(),
            message: "test".to_string(),
            refs: vec![],
            parent_hashes: vec![],
        });
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.commit_detail_commit.is_none());
    }

    #[test]
    fn test_commit_detail_panel_switch() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        app.commit_detail_panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.commit_detail_panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.commit_detail_panel, Panel::Left);
    }
}
```

- [ ] **Step 4: Verify all 3 files were created**

```bash
wc -l crates/gitat-ui/src/event/normal.rs crates/gitat-ui/src/event/staging.rs crates/gitat-ui/src/event/commit_detail.rs
```

Expected: normal.rs ~140, staging.rs ~170, commit_detail.rs ~170 lines.

---

### Task 4: Swap `event.rs` for `event/` module

**Files:**
- Delete: `crates/gitat-ui/src/event.rs`

- [ ] **Step 1: Delete the old `event.rs`**

```bash
rm crates/gitat-ui/src/event.rs
```

The `lib.rs` already declares `pub mod event;`. Rust resolves this to either `event.rs` or `event/mod.rs`. After deleting `event.rs`, the compiler will find `event/mod.rs`.

- [ ] **Step 2: Run `cargo check` to verify compilation**

Run: `cargo check 2>&1`
Expected: compiles with no errors (warnings OK).

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1`
Expected: all 36 tests pass (32 existing + 4 new `is_staged` tests).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: split event.rs into event/ module and use StatusEntry::is_staged()

- Extract is_staged() method to StatusEntry in gitat-core
- Split event.rs into event/mod.rs, normal.rs, staging.rs, commit_detail.rs
- Add status message for diff reload failure in stage_or_unstage_hunk
- All 36 tests pass

Task: リファクタリングして"
```

---

### Task 5: Update TODO.md

**Files:**
- Modify: `TODO.md`

- [ ] **Step 1: Mark the 3 refactoring items as completed**

In `TODO.md`, change the refactoring section from:

```markdown
## リファクタリング

- [ ] `is_staged` 判定ロジックの重複排除 — `event.rs` 内の3箇所で同一パターンが繰り返されている。ヘルパー関数への抽出
- [ ] `event.rs` のモジュール分割 — キー処理、ステージング、diff ロード等の責務が1ファイルに集中（700行超）
- [ ] `stage_or_unstage_hunk` の diff リロード失敗時にステータスメッセージでユーザーに通知
```

To:

```markdown
## リファクタリング

- [x] `is_staged` 判定ロジックの重複排除 — `StatusEntry::is_staged()` メソッドとして `gitat-core` に抽出済み
- [x] `event.rs` のモジュール分割 — `event/mod.rs`, `normal.rs`, `staging.rs`, `commit_detail.rs` に分割済み
- [x] `stage_or_unstage_hunk` の diff リロード失敗時にステータスメッセージでユーザーに通知
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "docs: mark refactoring items as completed in TODO.md

Task: リファクタリングして"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`
Expected: all 36 tests pass, 0 failures.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1`
Expected: no errors (warnings about unused are acceptable for unrelated code).

- [ ] **Step 3: Verify file structure**

```bash
find crates/gitat-ui/src/event -type f | sort
```

Expected:
```
crates/gitat-ui/src/event/commit_detail.rs
crates/gitat-ui/src/event/mod.rs
crates/gitat-ui/src/event/normal.rs
crates/gitat-ui/src/event/staging.rs
```

And confirm the old file is gone:
```bash
test ! -f crates/gitat-ui/src/event.rs && echo "OK: event.rs removed"
```
