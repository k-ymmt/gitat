# Search Mode Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add j/k navigation, preview sync, and direct CommitDetail access to search mode, with a mode stack for clean transitions.

**Architecture:** Add `mode_stack: Vec<Mode>` to `App` with `push_mode`/`pop_mode` helpers. Refactor `load_commit_preview` and `enter_commit_detail` to accept explicit `log_entries` indices (fixing a latent bug with filtered-mode index resolution). Extend `handle_search` with j/k, preview loading, and Enter->CommitDetail via `push_mode`.

**Tech Stack:** Rust, ratatui, crossterm, insta (snapshot testing)

---

### Task 1: Add mode stack to App

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:65-129` (App struct + new())
- Test: `crates/gitat-ui/src/app.rs` (inline tests module)

- [ ] **Step 1: Write failing tests for push_mode and pop_mode**

Add these tests to the `mod tests` block in `crates/gitat-ui/src/app.rs`:

```rust
#[test]
fn test_push_mode_saves_current_to_stack() {
    let mut app = App::new();
    assert_eq!(app.mode, Mode::Normal);
    app.push_mode(Mode::Search { query: "test".into() });
    assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
    assert_eq!(app.mode_stack.len(), 1);
    assert_eq!(app.mode_stack[0], Mode::Normal);
}

#[test]
fn test_pop_mode_restores_previous() {
    let mut app = App::new();
    app.push_mode(Mode::Search { query: "test".into() });
    app.push_mode(Mode::CommitDetail);
    assert_eq!(app.mode, Mode::CommitDetail);
    app.pop_mode();
    assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
    app.pop_mode();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.mode_stack.is_empty());
}

#[test]
fn test_pop_mode_noop_on_empty_stack() {
    let mut app = App::new();
    app.mode = Mode::CommitDetail;
    app.pop_mode();
    assert_eq!(app.mode, Mode::CommitDetail);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_push_mode_saves_current_to_stack test_pop_mode_restores_previous test_pop_mode_noop_on_empty_stack`
Expected: FAIL — `mode_stack`, `push_mode`, `pop_mode` do not exist

- [ ] **Step 3: Add mode_stack field and push_mode/pop_mode methods**

In `crates/gitat-ui/src/app.rs`, add the field to `App` struct after `pub mode: Mode,`:

```rust
pub mode_stack: Vec<Mode>,
```

In `App::new()`, initialize it:

```rust
mode_stack: Vec::new(),
```

Add methods to the `impl App` block:

```rust
pub fn push_mode(&mut self, mode: Mode) {
    let current = std::mem::replace(&mut self.mode, mode);
    self.mode_stack.push(current);
}

pub fn pop_mode(&mut self) {
    if let Some(prev) = self.mode_stack.pop() {
        self.mode = prev;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-ui test_push_mode_saves_current_to_stack test_pop_mode_restores_previous test_pop_mode_noop_on_empty_stack`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat(search): add mode_stack with push_mode/pop_mode to App"
```

---

### Task 2: Refactor commit preview/enter to accept explicit log_entries index

Currently `load_commit_preview` and `enter_commit_detail` both do `i - 1` assuming the uncommitted row offset. This is wrong for filtered mode (index 0 can't load, index N maps to wrong commit). Refactor to accept an explicit `log_entries` index.

**Files:**
- Modify: `crates/gitat-ui/src/event/commit_detail.rs:7-58`
- Modify: `crates/gitat-ui/src/event/mod.rs:23-36`
- Modify: `crates/gitat-ui/src/event/normal.rs:117-127`
- Test: `crates/gitat-ui/src/event/commit_detail.rs` (inline tests)

- [ ] **Step 1: Write a failing test for filtered-mode preview at index 0**

This test proves the current bug: in filtered mode, selecting index 0 should load a preview but currently doesn't. Add to `mod tests` in `crates/gitat-ui/src/event/commit_detail.rs`:

```rust
#[test]
fn test_load_commit_preview_filtered_mode_index_zero() {
    use super::super::load_log_preview;

    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa111".to_string(),
            short_hash: "aaa".to_string(),
            author: "Alice".to_string(),
            date: "2026-01-01".to_string(),
            message: "first".to_string(),
            refs: vec![],
            parent_hashes: vec!["p1".to_string()],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb222".to_string(),
            short_hash: "bbb".to_string(),
            author: "Bob".to_string(),
            date: "2026-01-02".to_string(),
            message: "second".to_string(),
            refs: vec![],
            parent_hashes: vec!["p2".to_string()],
        },
    ];
    // Simulate filtered mode: only the second commit matches
    app.filtered_log_indices = Some(vec![1]);
    app.log_list_state.select(Some(0)); // first item in filtered list

    let runner = MockRunner::new().with_response(
        "diff-tree --no-commit-id -r --name-status p2 bbb222",
        "M\tlib.rs\n",
    );

    load_log_preview(&mut app, &runner);

    // Should load the second commit (index 1 in log_entries)
    assert!(app.commit_detail_commit.is_some());
    assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");
    assert_eq!(app.commit_detail_files.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_load_commit_preview_filtered_mode_index_zero`
Expected: FAIL — `commit_detail_commit` is `None` because `load_commit_preview` does `i > 0` check and index 0 returns early

- [ ] **Step 3: Refactor load_commit_preview and enter_commit_detail**

In `crates/gitat-ui/src/event/commit_detail.rs`, add index-accepting variants and make the old functions delegate:

```rust
pub(super) fn load_commit_preview_at(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) {
    let commit = match app.log_entries.get(log_entry_index) {
        Some(c) => c.clone(),
        None => return,
    };

    let first_parent = commit.parent_hashes.first().map(|s| s.as_str());
    let files =
        match gitat_core::commit_detail::get_commit_files(runner, &commit.hash, first_parent) {
            Ok(f) => f,
            Err(_) => return,
        };

    app.commit_detail_files = files;
    app.commit_detail_commit = Some(commit);
}

pub(super) fn load_commit_preview(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    load_commit_preview_at(app, runner, idx);
}

pub(super) fn enter_commit_detail_at(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) {
    let commit = match app.log_entries.get(log_entry_index) {
        Some(c) => c.clone(),
        None => return,
    };

    let first_parent = commit.parent_hashes.first().map(|s| s.as_str());
    let files =
        match gitat_core::commit_detail::get_commit_files(runner, &commit.hash, first_parent) {
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
    app.commit_detail_diff_state = UnifiedDiffState::new();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    app.mode = Mode::CommitDetail;
}

pub(super) fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    enter_commit_detail_at(app, runner, idx);
}
```

- [ ] **Step 4: Update load_log_preview to resolve filtered indices**

In `crates/gitat-ui/src/event/mod.rs`, replace `load_log_preview`:

```rust
pub fn load_log_preview(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(ref indices) = app.filtered_log_indices {
        // In filtered mode: resolve through filtered_log_indices
        if let Some(sel) = app.log_list_state.selected() {
            if let Some(&log_idx) = indices.get(sel) {
                commit_detail::load_commit_preview_at(app, runner, log_idx);
            }
        }
    } else {
        match app.log_list_state.selected() {
            Some(0) => uncommitted_detail::load_uncommitted_preview(app, runner),
            Some(_) => commit_detail::load_commit_preview(app, runner),
            None => {}
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS (including existing tests and new test)

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/mod.rs
git commit -m "refactor(search): extract index-based commit preview/enter helpers, fix filtered mode preview"
```

---

### Task 3: Add j/k navigation and preview sync to search mode

**Files:**
- Modify: `crates/gitat-ui/src/event/mod.rs:16-17,98-139`
- Test: `crates/gitat-ui/src/event/mod.rs` (inline tests)

- [ ] **Step 1: Write failing tests for j/k navigation**

Add to `mod tests` in `crates/gitat-ui/src/event/mod.rs`:

```rust
#[test]
fn test_search_j_moves_cursor_down() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix bug".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "fix typo".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.mode = Mode::Search { query: "fix".into() };
    app.update_search_filter("fix");
    // filtered_log_indices = Some([0, 1]), selection = 0
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
    assert_eq!(app.log_list_state.selected(), Some(1));
    // Verify still in search mode
    assert!(matches!(app.mode, Mode::Search { ref query } if query == "fix"));
}

#[test]
fn test_search_k_moves_cursor_up() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix bug".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "fix typo".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.mode = Mode::Search { query: "fix".into() };
    app.update_search_filter("fix");
    app.log_list_state.select(Some(1)); // start at second item
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Char('k')), &runner);
    assert_eq!(app.log_list_state.selected(), Some(0));
}

#[test]
fn test_search_j_clamps_at_end() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.mode = Mode::Search { query: "fix".into() };
    app.update_search_filter("fix");
    // Only one result, selection at 0
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
    assert_eq!(app.log_list_state.selected(), Some(0)); // clamped
}

#[test]
fn test_search_k_clamps_at_start() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.mode = Mode::Search { query: "fix".into() };
    app.update_search_filter("fix");
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Char('k')), &runner);
    assert_eq!(app.log_list_state.selected(), Some(0)); // clamped at 0
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_search_j_moves_cursor_down test_search_k_moves_cursor_up test_search_j_clamps_at_end test_search_k_clamps_at_start`
Expected: FAIL — j/k chars are consumed by the `Char(c)` arm and appended to query instead of navigating

- [ ] **Step 3: Add runner parameter to handle_search and implement j/k**

In `crates/gitat-ui/src/event/mod.rs`, change the `handle_search` call site and function:

Update the `handle_key` dispatch (line 17):

```rust
Mode::Search { .. } => handle_search(app, key, runner),
```

Replace `handle_search` function with:

```rust
fn handle_search(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            if let Some(cursor) = app.pre_search_cursor {
                app.log_list_state.select(Some(cursor));
            }
            app.filtered_log_indices = None;
            app.pre_search_cursor = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            // Resolve filtered selection to original log_entries index
            let original_index = app.log_list_state.selected().and_then(|sel| {
                app.filtered_log_indices
                    .as_ref()
                    .and_then(|indices| indices.get(sel).copied())
            });
            app.filtered_log_indices = None;
            app.pre_search_cursor = None;
            app.mode = Mode::Normal;
            // Set cursor to original index + 1 (offset for uncommitted row)
            if let Some(idx) = original_index {
                app.log_list_state.select(Some(idx + 1));
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.filtered_log_indices.as_ref().map_or(0, |v| v.len());
            if max > 0 {
                let current = app.log_list_state.selected().unwrap_or(0);
                let next = (current + 1).min(max - 1);
                app.log_list_state.select(Some(next));
            }
            load_log_preview(app, runner);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let current = app.log_list_state.selected().unwrap_or(0);
            app.log_list_state.select(Some(current.saturating_sub(1)));
            load_log_preview(app, runner);
        }
        KeyCode::Backspace => {
            if let Mode::Search { query } = &mut app.mode {
                query.pop();
                let q = query.clone();
                app.update_search_filter(&q);
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Search { query } = &mut app.mode {
                query.push(c);
                let q = query.clone();
                app.update_search_filter(&q);
            }
        }
        _ => {}
    }
}
```

Note: j/k arms are placed BEFORE the `Char(c)` catch-all so they take priority.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/event/mod.rs
git commit -m "feat(search): add j/k navigation with preview sync in search mode"
```

---

### Task 4: Enter opens CommitDetail from search with push_mode

**Files:**
- Modify: `crates/gitat-ui/src/event/mod.rs` (handle_search Enter arm)
- Test: `crates/gitat-ui/src/event/mod.rs` (inline tests)

- [ ] **Step 1: Write failing test for Enter->CommitDetail**

Add to `mod tests` in `crates/gitat-ui/src/event/mod.rs`:

```rust
#[test]
fn test_search_enter_opens_commit_detail() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa111".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "first".into(),
            refs: vec![],
            parent_hashes: vec!["p1".into()],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb222".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "second".into(),
            refs: vec![],
            parent_hashes: vec!["p2".into()],
        },
    ];
    app.pre_search_cursor = Some(0);
    app.mode = Mode::Search { query: "second".into() };
    app.update_search_filter("second");
    // filtered_log_indices = Some([1]), selection = 0

    let runner = MockRunner::new()
        .with_response(
            "diff-tree --no-commit-id -r --name-status p2 bbb222",
            "M\tsrc/main.rs\n",
        )
        .with_response("diff p2..bbb222 -- src/main.rs", "");

    handle_key(&mut app, mock_key(KeyCode::Enter), &runner);

    // Should be in CommitDetail mode
    assert_eq!(app.mode, Mode::CommitDetail);
    // Search state preserved on stack
    assert_eq!(app.mode_stack.len(), 1);
    assert!(matches!(&app.mode_stack[0], Mode::Search { ref query } if query == "second"));
    // Filter and pre_search_cursor preserved
    assert_eq!(app.filtered_log_indices, Some(vec![1]));
    assert_eq!(app.pre_search_cursor, Some(0));
    // Commit detail data loaded
    assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");
}

#[test]
fn test_search_enter_noop_on_empty_results() {
    let mut app = App::new();
    app.log_entries = vec![];
    app.mode = Mode::Search { query: "nothing".into() };
    app.filtered_log_indices = Some(vec![]);
    app.log_list_state.select(None);
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Enter), &runner);

    // Should remain in search mode
    assert!(matches!(app.mode, Mode::Search { .. }));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_search_enter_opens_commit_detail test_search_enter_noop_on_empty_results`
Expected: FAIL — Enter currently clears filter and returns to Normal

- [ ] **Step 3: Update existing Enter test**

The existing `test_search_enter_confirms_selection` test expects the old behavior (Enter → Normal). Remove it since it tests the behavior being replaced:

Delete the `test_search_enter_confirms_selection` test from `crates/gitat-ui/src/event/mod.rs`.

- [ ] **Step 4: Implement Enter->CommitDetail with push_mode**

In `crates/gitat-ui/src/event/mod.rs`, replace the `KeyCode::Enter` arm in `handle_search`:

```rust
KeyCode::Enter => {
    let original_index = app.log_list_state.selected().and_then(|sel| {
        app.filtered_log_indices
            .as_ref()
            .and_then(|indices| indices.get(sel).copied())
    });
    if let Some(idx) = original_index {
        // Save current search selection for restoration
        let search_sel = app.log_list_state.selected();
        commit_detail::enter_commit_detail_at(app, runner, idx);
        // enter_commit_detail_at sets mode to CommitDetail directly;
        // we need to push Search onto the stack instead
        if app.mode == Mode::CommitDetail {
            // Replace the mode set by enter_commit_detail_at:
            // put Search back, then push_mode to CommitDetail properly
            let query = String::new(); // placeholder, will be overwritten
            app.mode = Mode::Search { query };
            // Restore the actual Search mode from before enter_commit_detail_at changed it
            // Since enter_commit_detail_at set mode to CommitDetail, we need a different approach.
        }
    }
}
```

Actually, the cleaner approach: call the data-loading parts of `enter_commit_detail_at` without the `app.mode = Mode::CommitDetail` line, then use `push_mode`. Refactor `enter_commit_detail_at` to not set the mode, and have callers set it:

In `crates/gitat-ui/src/event/commit_detail.rs`, rename `enter_commit_detail_at` to `prepare_commit_detail` and remove the `app.mode = Mode::CommitDetail;` line. Return `bool` indicating success:

```rust
/// Load commit detail data for the given log_entries index.
/// Returns true if data was loaded successfully. Does NOT change app.mode.
pub(super) fn prepare_commit_detail(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) -> bool {
    let commit = match app.log_entries.get(log_entry_index) {
        Some(c) => c.clone(),
        None => return false,
    };

    let first_parent = commit.parent_hashes.first().map(|s| s.as_str());
    let files =
        match gitat_core::commit_detail::get_commit_files(runner, &commit.hash, first_parent) {
            Ok(f) => f,
            Err(e) => {
                app.set_status_message(format!("Failed to load commit files: {e}"));
                return false;
            }
        };

    app.commit_detail_commit = Some(commit);
    app.commit_detail_files = files;
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_panel = Panel::Left;
    app.commit_detail_diff_state = UnifiedDiffState::new();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    true
}

pub(super) fn enter_commit_detail_at(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) {
    if prepare_commit_detail(app, runner, log_entry_index) {
        app.mode = Mode::CommitDetail;
    }
}

pub(super) fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    enter_commit_detail_at(app, runner, idx);
}
```

Then in `handle_search` Enter arm in `crates/gitat-ui/src/event/mod.rs`:

```rust
KeyCode::Enter => {
    let original_index = app.log_list_state.selected().and_then(|sel| {
        app.filtered_log_indices
            .as_ref()
            .and_then(|indices| indices.get(sel).copied())
    });
    if let Some(idx) = original_index {
        if commit_detail::prepare_commit_detail(app, runner, idx) {
            app.push_mode(Mode::CommitDetail);
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-ui/src/event/mod.rs crates/gitat-ui/src/event/commit_detail.rs
git commit -m "feat(search): Enter in search mode opens CommitDetail via push_mode"
```

---

### Task 5: CommitDetail exit uses pop_mode, Search Esc clears stack

**Files:**
- Modify: `crates/gitat-ui/src/event/commit_detail.rs:91-98` (handle_commit_detail Esc)
- Modify: `crates/gitat-ui/src/event/mod.rs` (handle_search Esc)
- Test: `crates/gitat-ui/src/event/mod.rs` and `crates/gitat-ui/src/event/commit_detail.rs`

- [ ] **Step 1: Write failing test for CommitDetail->Search return**

Add to `mod tests` in `crates/gitat-ui/src/event/commit_detail.rs`:

```rust
#[test]
fn test_esc_from_commit_detail_returns_to_search() {
    let mut app = App::new();
    // Simulate: was in Search, pushed to CommitDetail
    app.mode = Mode::CommitDetail;
    app.mode_stack = vec![Mode::Search { query: "test".into() }];
    app.filtered_log_indices = Some(vec![0, 2]);
    app.pre_search_cursor = Some(5);

    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

    assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
    assert_eq!(app.filtered_log_indices, Some(vec![0, 2]));
    assert_eq!(app.pre_search_cursor, Some(5));
}

#[test]
fn test_esc_from_commit_detail_falls_back_to_normal() {
    let mut app = App::new();
    app.mode = Mode::CommitDetail;
    // Empty stack — entered from Normal mode directly
    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
    assert_eq!(app.mode, Mode::Normal);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_esc_from_commit_detail_returns_to_search test_esc_from_commit_detail_falls_back_to_normal`
Expected: First test FAILS (returns to Normal instead of Search). Second test should PASS (current behavior already goes to Normal).

Note: `test_esc_from_commit_detail_falls_back_to_normal` will fail to compile until `mode_stack` field is added to the struct literal — but we already did that in Task 1. It will fail because the existing `test_esc_from_commit_detail_returns_to_normal` already exists and the new test has a conflicting name — use the name above which is different.

- [ ] **Step 3: Implement pop_mode in CommitDetail Esc handler**

In `crates/gitat-ui/src/event/commit_detail.rs`, replace the `Esc` arm in `handle_commit_detail`:

```rust
KeyCode::Esc => {
    app.pop_mode();
    // If stack was empty, pop_mode is no-op, fall back to Normal
    if matches!(app.mode, Mode::CommitDetail) {
        app.mode = Mode::Normal;
    }
    // Reset interactive state; keep data for preview
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_diff_state = UnifiedDiffState::new();
}
```

- [ ] **Step 4: Add mode_stack.clear() to Search Esc handler**

In `crates/gitat-ui/src/event/mod.rs`, update the `Esc` arm in `handle_search`:

```rust
KeyCode::Esc => {
    if let Some(cursor) = app.pre_search_cursor {
        app.log_list_state.select(Some(cursor));
    }
    app.filtered_log_indices = None;
    app.pre_search_cursor = None;
    app.mode = Mode::Normal;
    app.mode_stack.clear();
}
```

- [ ] **Step 5: Write test for Esc from search clears stack**

Add to `mod tests` in `crates/gitat-ui/src/event/mod.rs`:

```rust
#[test]
fn test_search_esc_clears_mode_stack() {
    let mut app = App::new();
    app.mode = Mode::Search { query: "test".into() };
    app.pre_search_cursor = Some(3);
    app.filtered_log_indices = Some(vec![0]);
    app.mode_stack = vec![Mode::Normal]; // leftover from some transition
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

    assert_eq!(app.mode, Mode::Normal);
    assert!(app.mode_stack.is_empty());
    assert_eq!(app.filtered_log_indices, None);
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
git add crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/mod.rs
git commit -m "feat(search): CommitDetail exit uses pop_mode, Search Esc clears stack"
```

---

### Task 6: Final integration test and cleanup

**Files:**
- Test: `crates/gitat-ui/src/event/mod.rs`
- Modify: `TODO.md`

- [ ] **Step 1: Write full round-trip integration test**

Add to `mod tests` in `crates/gitat-ui/src/event/mod.rs`:

```rust
#[test]
fn test_search_to_commit_detail_round_trip() {
    // Full flow: Normal -> Search -> navigate -> Enter -> CommitDetail -> Esc -> Search -> Esc -> Normal
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa111".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix bug".into(),
            refs: vec![],
            parent_hashes: vec!["p1".into()],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb222".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "fix typo".into(),
            refs: vec![],
            parent_hashes: vec!["p2".into()],
        },
    ];
    app.log_list_state.select(Some(1)); // some position in Normal mode

    let runner_search = MockRunner::new();

    // Step 1: Enter search mode
    handle_key(&mut app, mock_key(KeyCode::Char('/')), &runner_search);
    assert!(matches!(app.mode, Mode::Search { .. }));
    assert_eq!(app.pre_search_cursor, Some(1));

    // Step 2: Type "fix" — both match
    handle_key(&mut app, mock_key(KeyCode::Char('f')), &runner_search);
    handle_key(&mut app, mock_key(KeyCode::Char('i')), &runner_search);
    handle_key(&mut app, mock_key(KeyCode::Char('x')), &runner_search);
    assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));
    assert_eq!(app.log_list_state.selected(), Some(0));

    // Step 3: j to move to second result
    handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner_search);
    assert_eq!(app.log_list_state.selected(), Some(1));

    // Step 4: Enter to open CommitDetail for second result (bbb222)
    let runner_detail = MockRunner::new()
        .with_response(
            "diff-tree --no-commit-id -r --name-status p2 bbb222",
            "M\tlib.rs\n",
        )
        .with_response("diff p2..bbb222 -- lib.rs", "");
    handle_key(&mut app, mock_key(KeyCode::Enter), &runner_detail);
    assert_eq!(app.mode, Mode::CommitDetail);
    assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");

    // Step 5: Esc from CommitDetail -> back to Search
    let runner_back = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner_back);
    assert!(matches!(app.mode, Mode::Search { ref query } if query == "fix"));
    assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));

    // Step 6: Esc from Search -> back to Normal
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner_back);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.filtered_log_indices, None);
    assert_eq!(app.log_list_state.selected(), Some(1)); // restored
    assert!(app.mode_stack.is_empty());
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS

- [ ] **Step 3: Run full test suite and clippy**

Run: `cargo test && cargo clippy`
Expected: ALL PASS, no warnings

- [ ] **Step 4: Update TODO.md**

Mark the three search mode improvement items as done:

```markdown
## 検索モードの改善

- [x] 検索結果内での j/k ナビゲーション — 検索モード中にカーソルを上下移動してフィルタ結果を選択できるようにする
- [x] 検索結果内でのプレビュー連動 — フィルタモードでカーソル移動時にプレビューパネルを更新する
- [x] 検索結果から直接コミット詳細画面へ遷移 — Enter でフィルタ解除+カーソル移動だけでなく、直接 CommitDetail モードに入れるオプション
```

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/event/mod.rs TODO.md
git commit -m "feat(search): complete search mode improvements — j/k nav, preview sync, CommitDetail access"
```
