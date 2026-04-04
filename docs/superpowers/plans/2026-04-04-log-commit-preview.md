# Log Commit Preview Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an inline commit detail preview to the bottom 40% of the Log tab, showing file list + diff for the selected commit in read-only mode.

**Architecture:** Reuse existing `commit_detail_*` and `status`/`current_diff` App fields for preview data. Extract data loading into reusable helpers called on cursor move. Split `render_log_list()` vertically (60/40) and add read-only preview renderers.

**Tech Stack:** Rust, ratatui 0.30.x, crossterm

---

### Task 1: Extract preview data loading functions

**Files:**
- Modify: `crates/gitat-ui/src/event/commit_detail.rs`
- Modify: `crates/gitat-ui/src/event/uncommitted_detail.rs`
- Modify: `crates/gitat-ui/src/event/mod.rs`

- [ ] **Step 1: Write test for load_commit_preview**

Add to the `#[cfg(test)] mod tests` in `crates/gitat-ui/src/event/commit_detail.rs`:

```rust
#[test]
fn test_load_commit_preview_populates_data() {
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
    app.log_list_state.select(Some(1)); // index 1 = first commit

    let runner = MockRunner::new()
        .with_response(
            "diff-tree --no-commit-id -r --name-status abc123",
            "M\tsrc/main.rs\n",
        )
        .with_response("diff parent1..abc123 -- src/main.rs", "");

    load_commit_preview(&mut app, &runner);

    assert!(app.commit_detail_commit.is_some());
    assert_eq!(app.commit_detail_files.len(), 1);
    assert_eq!(app.commit_detail_files[0].path, "src/main.rs");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_load_commit_preview_populates_data`
Expected: FAIL with `cannot find function load_commit_preview`

- [ ] **Step 3: Implement load_commit_preview**

Add this function in `crates/gitat-ui/src/event/commit_detail.rs`, before `enter_commit_detail`:

```rust
pub(super) fn load_commit_preview(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    let commit = match app.log_entries.get(idx) {
        Some(c) => c.clone(),
        None => return,
    };

    let files = match gitat_core::commit_detail::get_commit_files(runner, &commit.hash) {
        Ok(f) => f,
        Err(_) => return,
    };

    app.commit_detail_files = files;

    // Load diff for first file
    if let Some(file_entry) = app.commit_detail_files.first() {
        let parent = commit.parent_hashes.first().map(|s| s.as_str());
        if let Ok(diff) = gitat_core::commit_detail::get_commit_file_diff(
            runner,
            &commit.hash,
            parent,
            &file_entry.path,
        ) {
            app.commit_detail_diff = Some(diff);
        }
    }

    app.commit_detail_commit = Some(commit);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_load_commit_preview_populates_data`
Expected: PASS

- [ ] **Step 5: Write test for load_uncommitted_preview**

Add to `#[cfg(test)] mod tests` in `crates/gitat-ui/src/event/uncommitted_detail.rs`:

```rust
#[test]
fn test_load_uncommitted_preview_populates_diff() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.status = vec![gitat_core::status::StatusEntry {
        path: "src/main.rs".to_string(),
        index_status: gitat_core::status::FileStatus::Modified,
        worktree_status: gitat_core::status::FileStatus::Unmodified,
    }];

    let runner = MockRunner::new()
        .with_response("diff --cached -- src/main.rs", "");

    load_uncommitted_preview(&mut app, &runner);

    assert!(app.current_diff.is_some());
}

#[test]
fn test_load_uncommitted_preview_empty_status() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.status = vec![];

    let runner = MockRunner::new();
    load_uncommitted_preview(&mut app, &runner);

    assert!(app.current_diff.is_none());
}
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_load_uncommitted_preview`
Expected: FAIL with `cannot find function load_uncommitted_preview`

- [ ] **Step 7: Implement load_uncommitted_preview**

Add this function in `crates/gitat-ui/src/event/uncommitted_detail.rs`, before `enter_uncommitted_detail`:

```rust
pub(super) fn load_uncommitted_preview(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(entry) = app.status.first() {
        let staged = entry.is_staged();
        if let Ok(diff) = gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
            app.current_diff = Some(diff);
        }
    } else {
        app.current_diff = None;
    }
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p gitat-ui test_load_uncommitted_preview`
Expected: PASS

- [ ] **Step 9: Add public dispatcher in event/mod.rs**

Add this public function in `crates/gitat-ui/src/event/mod.rs`, after the `handle_key` function:

```rust
pub fn load_log_preview(app: &mut App, runner: &dyn CommandRunner) {
    match app.log_list_state.selected() {
        Some(0) => uncommitted_detail::load_uncommitted_preview(app, runner),
        Some(_) => commit_detail::load_commit_preview(app, runner),
        None => {}
    }
}
```

- [ ] **Step 10: Run all tests to verify nothing is broken**

Run: `cargo test -p gitat-ui`
Expected: All tests PASS

- [ ] **Step 11: Commit**

```bash
git add crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/uncommitted_detail.rs crates/gitat-ui/src/event/mod.rs
git commit -m "feat: extract preview data loading functions for log commit preview"
```

---

### Task 2: Preserve preview data on Esc from full-screen modes

**Files:**
- Modify: `crates/gitat-ui/src/event/commit_detail.rs`
- Modify: `crates/gitat-ui/src/event/uncommitted_detail.rs`

- [ ] **Step 1: Write test for CommitDetail Esc preserving data**

Add to `#[cfg(test)] mod tests` in `crates/gitat-ui/src/event/commit_detail.rs`:

```rust
#[test]
fn test_esc_from_commit_detail_preserves_preview_data() {
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
    app.commit_detail_files = vec![gitat_core::commit_detail::CommitFileEntry {
        path: "src/main.rs".to_string(),
        status: gitat_core::commit_detail::FileChangeStatus::Modified,
    }];

    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

    assert_eq!(app.mode, Mode::Normal);
    // Data should be preserved for preview
    assert!(app.commit_detail_commit.is_some());
    assert_eq!(app.commit_detail_files.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_esc_from_commit_detail_preserves_preview_data`
Expected: FAIL because current Esc clears `commit_detail_commit` and `commit_detail_files`

- [ ] **Step 3: Modify CommitDetail Esc handler**

In `crates/gitat-ui/src/event/commit_detail.rs`, change the `KeyCode::Esc` arm in `handle_commit_detail` from:

```rust
KeyCode::Esc => {
    app.mode = Mode::Normal;
    app.commit_detail_commit = None;
    app.commit_detail_files.clear();
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_diff = None;
    app.commit_detail_diff_state = SideBySideDiffState::new();
}
```

to:

```rust
KeyCode::Esc => {
    app.mode = Mode::Normal;
    // Reset interactive state only; keep data for preview
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_diff_state = SideBySideDiffState::new();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_esc_from_commit_detail_preserves_preview_data`
Expected: PASS

- [ ] **Step 5: Update existing Esc test**

The existing `test_esc_from_commit_detail_returns_to_normal` asserts `app.commit_detail_commit.is_none()`. Update it to assert mode change only:

In `crates/gitat-ui/src/event/commit_detail.rs`, change:

```rust
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
```

to:

```rust
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
}
```

- [ ] **Step 6: Write test for UncommittedDetail Esc preserving diff**

Add to `#[cfg(test)] mod tests` in `crates/gitat-ui/src/event/uncommitted_detail.rs`:

```rust
#[test]
fn test_esc_from_uncommitted_detail_preserves_diff() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.mode = Mode::UncommittedDetail;
    app.current_diff = Some(vec![]);

    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

    assert_eq!(app.mode, Mode::Normal);
    // Diff data should be preserved for preview
    assert!(app.current_diff.is_some());
}
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_esc_from_uncommitted_detail_preserves_diff`
Expected: FAIL because current Esc sets `app.current_diff = None`

- [ ] **Step 8: Modify UncommittedDetail Esc handler**

In `crates/gitat-ui/src/event/uncommitted_detail.rs`, change the `KeyCode::Esc` arm in `handle_uncommitted_detail` from:

```rust
KeyCode::Esc => {
    app.mode = Mode::Normal;
    app.current_diff = None;
    app.diff_state = SideBySideDiffState::new();
}
```

to:

```rust
KeyCode::Esc => {
    app.mode = Mode::Normal;
    // Reset interactive state only; keep diff data for preview
    app.diff_state = SideBySideDiffState::new();
}
```

- [ ] **Step 9: Run all tests to verify everything passes**

Run: `cargo test -p gitat-ui`
Expected: All tests PASS

- [ ] **Step 10: Commit**

```bash
git add crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/uncommitted_detail.rs
git commit -m "feat: preserve preview data on Esc from full-screen detail modes"
```

---

### Task 3: Load preview data on cursor move and initial display

**Files:**
- Modify: `crates/gitat-ui/src/event/normal.rs`
- Modify: `crates/gitat/src/main.rs`

- [ ] **Step 1: Write test for cursor move triggering preview load**

Add to `#[cfg(test)] mod tests` in `crates/gitat-ui/src/event/normal.rs` (create the test module if it doesn't exist — there is no existing test module in this file):

```rust
#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use crate::app::{App, Mode, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_j_on_log_tab_loads_commit_preview() {
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
        // No selection yet; first j press selects index 0 (uncommitted)
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(0));

        // Second j press selects index 1 (first commit) and loads preview
        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(1));
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_j_on_log_tab_loads_commit_preview`
Expected: FAIL because `commit_detail_commit` is None (no preview loading on cursor move yet)

- [ ] **Step 3: Add preview loading after cursor move in normal.rs**

In `crates/gitat-ui/src/event/normal.rs`, modify the `KeyCode::Char('j') | KeyCode::Down` arm. After the existing cursor move logic, add preview loading:

```rust
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
    if app.tab == Tab::Log {
        super::load_log_preview(app, runner);
    }
}
```

Do the same for the `KeyCode::Char('k') | KeyCode::Up` arm:

```rust
KeyCode::Char('k') | KeyCode::Up => {
    let state = app.current_list_state_mut();
    if let Some(i) = state.selected() {
        let next = if i == 0 { 0 } else { i - 1 };
        state.select(Some(next));
    }
    if app.tab == Tab::Log {
        super::load_log_preview(app, runner);
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_j_on_log_tab_loads_commit_preview`
Expected: PASS

- [ ] **Step 5: Add initial selection and preview load in main.rs**

In `crates/gitat/src/main.rs`, after `app.refresh(&runner);` (line 30), add:

```rust
app.log_list_state.select(Some(0));
gitat_ui::event::load_log_preview(&mut app, &runner);
```

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/gitat-ui/src/event/normal.rs crates/gitat/src/main.rs
git commit -m "feat: load preview data on cursor move and initial display"
```

---

### Task 4: Split render_log_list layout and add commit preview rendering

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Split render_log_list into layout + list items**

In `crates/gitat-ui/src/views/log.rs`, rename the existing `render_log_list` function body into a new helper, and rewrite `render_log_list` to split the area:

Replace the entire `render_log_list` function with:

```rust
fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_log_list_items(f, app, chunks[0]);

    match app.log_list_state.selected() {
        Some(0) => render_uncommitted_preview(f, app, chunks[1]),
        Some(_) => render_commit_preview(f, app, chunks[1]),
        None => {}
    }
}
```

Then add the extracted `render_log_list_items` — this is the original `render_log_list` body:

```rust
fn render_log_list_items(f: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Uncommitted changes item (always at index 0)
    let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
    let unstaged_count = app.status.iter().filter(|e| {
        !e.is_staged() && e.worktree_status != FileStatus::Untracked
    }).count();
    let untracked_count = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Untracked
    }).count();
    let total_changes = staged_count + unstaged_count + untracked_count;

    let uncommitted_spans = if total_changes > 0 {
        vec![
            Span::styled("● ", Theme::border_focused()),
            Span::styled("Uncommitted Changes", Theme::border_focused()),
            Span::styled(
                format!(" — {} staged, {} unstaged", staged_count, unstaged_count + untracked_count),
                Theme::diff_context(),
            ),
        ]
    } else {
        vec![
            Span::styled("● ", Theme::border_focused()),
            Span::styled("Uncommitted Changes", Theme::border_focused()),
        ]
    };
    items.push(ListItem::new(Line::from(uncommitted_spans)));

    // Log entries (index 1..N)
    for c in &app.log_entries {
        let mut spans = vec![
            Span::styled(&c.short_hash, Theme::commit_hash()),
            Span::raw(" "),
        ];
        if !c.refs.is_empty() {
            spans.push(Span::styled(
                format!("({}) ", c.refs.join(", ")),
                Theme::commit_ref(),
            ));
        }
        spans.push(Span::raw(&c.message));
        items.push(ListItem::new(Line::from(spans)));
    }

    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.log_list_state);
}
```

- [ ] **Step 2: Add import and render_commit_preview function**

First, update the import in `crates/gitat-ui/src/views/log.rs` from:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiff;
```

to:

```rust
use crate::widgets::side_by_side_diff::{SideBySideDiff, SideBySideDiffState};
```

Then add these functions in `crates/gitat-ui/src/views/log.rs`:

```rust
fn render_commit_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let commit = match &app.commit_detail_commit {
        Some(c) => c,
        None => return,
    };

    // Split: metadata (3 lines) + panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Metadata
    let meta_lines = vec![
        Line::from(vec![
            Span::styled(&commit.short_hash, Theme::commit_hash()),
            Span::raw("  "),
            Span::raw(&commit.author),
            Span::raw("  "),
            Span::styled(&commit.date, Theme::diff_context()),
        ]),
        Line::from(Span::raw(&commit.message)),
        Line::from(""),
    ];
    let meta = Paragraph::new(meta_lines)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Theme::border()));
    f.render_widget(meta, chunks[0]);

    // Two panels
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    render_commit_preview_files(f, app, panels[0]);
    render_commit_preview_diff(f, app, panels[1]);
}

fn render_commit_preview_files(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app.commit_detail_files.iter().map(|entry| {
        let (code, style) = match entry.status {
            FileChangeStatus::Added => ("A", Theme::file_added()),
            FileChangeStatus::Modified => ("M", Theme::commit_hash()),
            FileChangeStatus::Deleted => ("D", Theme::file_unstaged()),
            FileChangeStatus::Renamed => ("R", Theme::commit_ref()),
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{code} "), style),
            Span::raw(&entry.path),
        ]))
    }).collect();

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_commit_preview_diff(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref diff_files) = app.commit_detail_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let widget = SideBySideDiff::new(diff_files).block(block);
        let mut state = SideBySideDiffState::new();
        f.render_stateful_widget(widget, area, &mut state);
    } else {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let placeholder = Paragraph::new("No diff available")
            .block(block)
            .style(Theme::diff_context());
        f.render_widget(placeholder, area);
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p gitat`
Expected: Compiles (the `render_uncommitted_preview` function is used but not yet defined — add a stub)

Add a temporary stub at the bottom of `log.rs` to allow compilation:

```rust
fn render_uncommitted_preview(f: &mut Frame, app: &mut App, area: Rect) {
    // Placeholder — implemented in Task 5
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(Theme::border());
    let placeholder = Paragraph::new("Loading...")
        .block(block)
        .style(Theme::diff_context());
    f.render_widget(placeholder, area);
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/views/log.rs
git commit -m "feat: split log list layout and add commit preview rendering"
```

---

### Task 5: Add uncommitted preview rendering

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Replace the render_uncommitted_preview stub**

In `crates/gitat-ui/src/views/log.rs`, replace the `render_uncommitted_preview` stub with the full implementation:

```rust
fn render_uncommitted_preview(f: &mut Frame, app: &mut App, area: Rect) {
    // Split: header (1 line) + panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header
    let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
    let unstaged_count = app.status.iter().filter(|e| {
        !e.is_staged() && e.worktree_status != FileStatus::Untracked
    }).count();
    let untracked_count = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Untracked
    }).count();
    let total_changes = staged_count + unstaged_count + untracked_count;

    let header_text = if total_changes > 0 {
        format!(
            "Uncommitted Changes — {} staged, {} unstaged",
            staged_count,
            unstaged_count + untracked_count
        )
    } else {
        "Uncommitted Changes".to_string()
    };
    let header = Paragraph::new(Line::from(Span::styled(header_text, Theme::border_focused())))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Theme::border()));
    f.render_widget(header, chunks[0]);

    // Two panels
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    render_uncommitted_preview_files(f, app, panels[0]);
    render_uncommitted_preview_diff(f, app, panels[1]);
}

fn render_uncommitted_preview_files(f: &mut Frame, app: &App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Staged section
    let staged: Vec<_> = app.status.iter().filter(|e| {
        e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
    }).collect();

    if !staged.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Staged",
            Theme::file_staged(),
        ))));
        for entry in &staged {
            let code = file_status_code(&entry.index_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_staged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    // Modified (unstaged) section
    let modified: Vec<_> = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Unmodified
            && e.worktree_status != FileStatus::Unmodified
            && e.worktree_status != FileStatus::Untracked
    }).collect();

    if !modified.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Modified",
            Theme::file_unstaged(),
        ))));
        for entry in &modified {
            let code = file_status_code(&entry.worktree_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_unstaged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    // Untracked section
    let untracked: Vec<_> = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Untracked
    }).collect();

    if !untracked.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Untracked",
            Theme::file_untracked(),
        ))));
        for entry in &untracked {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("? ", Theme::file_untracked()),
                Span::raw(&entry.path),
            ])));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No uncommitted changes",
            Theme::file_untracked(),
        ))));
    }

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn render_uncommitted_preview_diff(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref diff_files) = app.current_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let widget = SideBySideDiff::new(diff_files).block(block);
        let mut state = SideBySideDiffState::new();
        f.render_stateful_widget(widget, area, &mut state);
    } else {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let placeholder = Paragraph::new("No diff available")
            .block(block)
            .style(Theme::diff_context());
        f.render_widget(placeholder, area);
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p gitat`
Expected: Compiles with no errors

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 4: Manual verification**

Run: `cargo run -p gitat` in a git repository.

Verify:
1. Log tab shows commit list in top 60%, preview in bottom 40%
2. Moving cursor (j/k) updates the preview to show selected commit's files and diff
3. Selecting "Uncommitted Changes" (index 0) shows uncommitted file list and diff
4. Pressing Enter opens full-screen CommitDetail/UncommittedDetail
5. Pressing Esc from full-screen returns to Normal with preview intact
6. Preview file list has no selection highlight
7. Preview diff is fixed at top (no scrolling)

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/views/log.rs
git commit -m "feat: add uncommitted preview rendering, complete log commit preview panel"
```
