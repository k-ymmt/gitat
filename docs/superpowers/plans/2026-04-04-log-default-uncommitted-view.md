# Log Default + Uncommitted Changes View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Status tab with a unified Log-centric workflow where the Log tab is the default screen and an "Uncommitted Changes" item at the top of the log list provides full staging/commit capabilities.

**Architecture:** Add `Mode::UncommittedDetail` parallel to `Mode::CommitDetail`. Migrate Status view rendering logic into `views/log.rs`. Remove `Tab::Status` and `views/status.rs`. The new uncommitted item at Log list index 0 opens a 2-column detail view with the same file list (Staged/Modified/Untracked) and diff viewer, plus staging and commit support.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.28, gitat-core (git operations)

---

### Task 1: Add `Mode::UncommittedDetail` and `uncommitted_list_state` to App

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:52-60` (Mode enum)
- Modify: `crates/gitat-ui/src/app.rs:68-91` (App struct)
- Modify: `crates/gitat-ui/src/app.rs:93-119` (App::new)

- [ ] **Step 1: Write failing test for Mode::UncommittedDetail**

Add to the `tests` module in `crates/gitat-ui/src/app.rs`:

```rust
#[test]
fn test_uncommitted_detail_mode_exists() {
    let mut app = App::new();
    app.mode = Mode::UncommittedDetail;
    assert_eq!(app.mode, Mode::UncommittedDetail);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_uncommitted_detail_mode_exists`
Expected: FAIL — `UncommittedDetail` variant does not exist on `Mode`

- [ ] **Step 3: Add `Mode::UncommittedDetail` variant and `uncommitted_list_state` field**

In `crates/gitat-ui/src/app.rs`, add `UncommittedDetail` to the `Mode` enum, and add `return_to_uncommitted_detail` flag for commit return:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Commit { message: String },
    Conflict { file: String },
    Search { query: String },
    Help,
    CommitDetail,
    UncommittedDetail,
}
```

Add `uncommitted_list_state` and `return_to_uncommitted_detail` fields to the `App` struct (after `status_list_state`):

```rust
pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub panel: Panel,
    pub should_quit: bool,
    pub status_list_state: ListState,
    pub uncommitted_list_state: ListState,
    pub log_list_state: ListState,
    // ... existing fields unchanged ...
    pub return_to_uncommitted_detail: bool,
}
```

Initialize in `App::new()`:

```rust
uncommitted_list_state: ListState::default(),
return_to_uncommitted_detail: false,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_uncommitted_detail_mode_exists`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat: add Mode::UncommittedDetail and uncommitted_list_state to App"
```

---

### Task 2: Create `event/uncommitted_detail.rs` with event handlers

**Files:**
- Create: `crates/gitat-ui/src/event/uncommitted_detail.rs`
- Modify: `crates/gitat-ui/src/event/mod.rs:1` (add module declaration)
- Modify: `crates/gitat-ui/src/event/mod.rs:11-18` (add match arm)

- [ ] **Step 1: Write failing test for entering UncommittedDetail mode**

Add a test file section at the end of `crates/gitat-ui/src/event/uncommitted_detail.rs` (the file will be created in step 3):

```rust
#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use crate::app::{App, Mode, Panel, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_esc_from_uncommitted_detail_returns_to_normal() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_uncommitted_detail_panel_switch() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn test_uncommitted_detail_c_enters_commit_mode() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
    }

    #[test]
    fn test_uncommitted_detail_q_quits() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('q')), &runner);
        assert!(app.should_quit);
    }
}
```

- [ ] **Step 2: Create the handler file and wire it into the event dispatcher**

Create `crates/gitat-ui/src/event/uncommitted_detail.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::side_by_side_diff::SideBySideDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn enter_uncommitted_detail(app: &mut App, runner: &dyn CommandRunner) {
    app.uncommitted_list_state = ratatui::widgets::ListState::default();
    app.panel = Panel::Left;
    app.diff_state = SideBySideDiffState::new();
    app.current_diff = None;

    if !app.status.is_empty() {
        app.uncommitted_list_state.select(Some(0));
        load_uncommitted_diff(app, runner);
    }
    app.mode = Mode::UncommittedDetail;
}

fn load_uncommitted_diff(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.uncommitted_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let staged = entry.is_staged();
    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.current_diff = Some(diff);
            app.diff_state = SideBySideDiffState::new();
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_uncommitted_detail(
    app: &mut App,
    key: KeyEvent,
    runner: &dyn CommandRunner,
) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.current_diff = None;
            app.diff_state = SideBySideDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.panel == Panel::Left {
                let len = app.status.len();
                if len > 0 {
                    let i = match app.uncommitted_list_state.selected() {
                        Some(i) => (i + 1).min(len - 1),
                        None => 0,
                    };
                    app.uncommitted_list_state.select(Some(i));
                    load_uncommitted_diff(app, runner);
                }
            } else {
                app.diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.panel == Panel::Left {
                if let Some(i) = app.uncommitted_list_state.selected() {
                    let next = if i == 0 { 0 } else { i - 1 };
                    app.uncommitted_list_state.select(Some(next));
                    load_uncommitted_diff(app, runner);
                }
            } else {
                app.diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('s') => {
            match app.panel {
                Panel::Left => super::staging::stage_or_unstage(app, runner),
                Panel::Right => super::staging::stage_or_unstage_hunk(app, runner),
            }
        }
        KeyCode::Char('c') => {
            app.return_to_uncommitted_detail = true;
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('J') => {
            app.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.diff_state.prev_hunk();
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use crate::app::{App, Mode, Panel, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_esc_from_uncommitted_detail_returns_to_normal() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_uncommitted_detail_panel_switch() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn test_uncommitted_detail_c_enters_commit_mode() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
    }

    #[test]
    fn test_uncommitted_detail_q_quits() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('q')), &runner);
        assert!(app.should_quit);
    }
}
```

Update `crates/gitat-ui/src/event/mod.rs` — add module declaration at line 1 and match arm:

```rust
mod commit_detail;
mod normal;
mod staging;
mod uncommitted_detail;
```

Update the `handle_key` match in `event/mod.rs`:

```rust
pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match app.mode {
        Mode::Normal => normal::handle_normal(app, key, runner),
        Mode::Commit { .. } => handle_commit(app, key, runner),
        Mode::Help => handle_help(app, key),
        Mode::Search { .. } => handle_search(app, key),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => commit_detail::handle_commit_detail(app, key, runner),
        Mode::UncommittedDetail => uncommitted_detail::handle_uncommitted_detail(app, key, runner),
    }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p gitat-ui uncommitted_detail`
Expected: All 4 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gitat-ui/src/event/uncommitted_detail.rs crates/gitat-ui/src/event/mod.rs
git commit -m "feat: add UncommittedDetail event handler with key bindings"
```

---

### Task 3: Update staging.rs to support UncommittedDetail mode

**Files:**
- Modify: `crates/gitat-ui/src/event/staging.rs:1-10` (imports and field references)
- Modify: `crates/gitat-ui/src/event/staging.rs:94-117` (load_diff_for_selected)

The staging functions currently use `app.status_list_state` to find the selected file. They need to also work when called from UncommittedDetail mode, using `app.uncommitted_list_state`.

- [ ] **Step 1: Write failing test for staging from UncommittedDetail mode**

Add to `crates/gitat-ui/src/event/staging.rs` tests:

```rust
#[test]
fn test_s_in_uncommitted_detail_stages_file() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.mode = Mode::UncommittedDetail;
    app.panel = Panel::Left;
    app.status = vec![gitat_core::status::StatusEntry {
        path: "src/main.rs".to_string(),
        index_status: gitat_core::status::FileStatus::Unmodified,
        worktree_status: gitat_core::status::FileStatus::Modified,
    }];
    app.uncommitted_list_state.select(Some(0));

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_s_in_uncommitted_detail_stages_file`
Expected: FAIL — staging uses `status_list_state` which has no selection in UncommittedDetail mode

- [ ] **Step 3: Update staging.rs to use the correct list state based on mode**

In `crates/gitat-ui/src/event/staging.rs`, add a helper function at the top and update `stage_or_unstage`, `stage_or_unstage_hunk`, and `load_diff_for_selected`:

Add import at top:

```rust
use crate::app::{App, Mode, Tab};
use crate::widgets::side_by_side_diff::SideBySideDiffState;
use gitat_core::runner::CommandRunner;
```

Add helper function:

```rust
fn selected_status_index(app: &App) -> Option<usize> {
    match app.mode {
        Mode::UncommittedDetail => app.uncommitted_list_state.selected(),
        _ => app.status_list_state.selected(),
    }
}
```

Update `stage_or_unstage` to use the helper:

```rust
pub(super) fn stage_or_unstage(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match selected_status_index(app) {
        Some(i) => i,
        None => return,
    };
    // ... rest unchanged
```

Update `stage_or_unstage_hunk` to use the helper:

```rust
pub(super) fn stage_or_unstage_hunk(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match selected_status_index(app) {
        Some(i) => i,
        None => return,
    };
    // ... rest unchanged
```

Update `load_diff_for_selected` to work in UncommittedDetail mode:

```rust
pub(super) fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if app.tab != Tab::Status && !matches!(app.mode, Mode::UncommittedDetail) {
        return;
    }
    let idx = match selected_status_index(app) {
        Some(i) => i,
        None => return,
    };
    // ... rest unchanged
```

Also add `Mode` to the existing import from `crate::app`:

```rust
use crate::app::{App, Mode, Tab};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-ui staging`
Expected: All staging tests PASS (including the new one)

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/event/staging.rs
git commit -m "feat: update staging.rs to support UncommittedDetail mode"
```

---

### Task 4: Update Log view with uncommitted item and UncommittedDetail rendering

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs` (full rewrite of render functions)

- [ ] **Step 1: Update `render()` to handle UncommittedDetail mode**

In `crates/gitat-ui/src/views/log.rs`, update the `render` function:

```rust
pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    match app.mode {
        Mode::CommitDetail => render_commit_detail(f, app, area),
        Mode::UncommittedDetail => render_uncommitted_detail(f, app, area),
        _ => render_log_list(f, app, area),
    }
}
```

Add `Mode` to the import from `crate::app`:

```rust
use crate::app::{App, Mode, Panel};
```

- [ ] **Step 2: Update `render_log_list` to include uncommitted item at index 0**

Replace the `render_log_list` function:

```rust
fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    // Uncommitted changes item (always at index 0)
    let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
    let unstaged_count = app.status.iter().filter(|e| {
        !e.is_staged() && e.worktree_status != gitat_core::status::FileStatus::Untracked
    }).count();
    let untracked_count = app.status.iter().filter(|e| {
        e.index_status == gitat_core::status::FileStatus::Untracked
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

Add import at the top of the file:

```rust
use gitat_core::status::FileStatus;
```

- [ ] **Step 3: Add `render_uncommitted_detail` function**

Add this function to `crates/gitat-ui/src/views/log.rs` (ported from `views/status.rs`):

```rust
fn render_uncommitted_detail(f: &mut Frame, app: &mut App, area: Rect) {
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

    render_uncommitted_file_list(f, app, panels[0]);
    render_uncommitted_diff(f, app, panels[1]);
}

fn file_status_code(status: &FileStatus) -> &'static str {
    match status {
        FileStatus::Modified => "M",
        FileStatus::Added => "A",
        FileStatus::Deleted => "D",
        FileStatus::Renamed => "R",
        FileStatus::Copied => "C",
        FileStatus::Untracked => "?",
        FileStatus::Unmodified => " ",
    }
}

fn render_uncommitted_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

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
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.uncommitted_list_state);
}

fn render_uncommitted_diff(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.panel == Panel::Right;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.current_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let widget = SideBySideDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.diff_state);
    } else {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let placeholder = Paragraph::new("Select a file to view diff")
            .block(block)
            .style(Theme::diff_context());
        f.render_widget(placeholder, area);
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p gitat-ui`
Expected: Compiles with no errors (warnings for unused status.rs are OK)

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/views/log.rs
git commit -m "feat: add uncommitted item to Log list and UncommittedDetail view rendering"
```

---

### Task 5: Update normal.rs — Enter dispatch, list_len, s key

**Files:**
- Modify: `crates/gitat-ui/src/event/normal.rs:119-126` (Enter key)
- Modify: `crates/gitat-ui/src/event/normal.rs:130-137` (list_len)
- Modify: `crates/gitat-ui/src/event/normal.rs:60-67` (s key)
- Modify: `crates/gitat-ui/src/event/commit_detail.rs:7-15` (enter_commit_detail offset)

- [ ] **Step 1: Write failing test for Enter on index 0 entering UncommittedDetail**

Add to `crates/gitat-ui/src/event/uncommitted_detail.rs` tests:

```rust
#[test]
fn test_enter_on_index_0_enters_uncommitted_detail() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.log_list_state.select(Some(0));
    // No log entries needed — index 0 is always the uncommitted item

    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
    assert!(matches!(app.mode, Mode::UncommittedDetail));
}

#[test]
fn test_enter_on_index_1_enters_commit_detail() {
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

    handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
    assert!(matches!(app.mode, Mode::CommitDetail));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_enter_on_index_0_enters_uncommitted_detail`
Expected: FAIL — current Enter handler calls `enter_commit_detail` for all Log tab entries

- [ ] **Step 3: Update Enter key handling in normal.rs**

In `crates/gitat-ui/src/event/normal.rs`, replace the Enter arm:

```rust
KeyCode::Enter => {
    if app.tab == Tab::Log {
        match app.log_list_state.selected() {
            Some(0) => super::uncommitted_detail::enter_uncommitted_detail(app, runner),
            Some(_) => super::commit_detail::enter_commit_detail(app, runner),
            None => {}
        }
    } else {
        super::staging::load_diff_for_selected(app, runner);
    }
}
```

- [ ] **Step 4: Update `list_len` to include uncommitted item for Log tab**

In `crates/gitat-ui/src/event/normal.rs`, update `list_len`:

```rust
fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::Status => app.status.len(),
        Tab::Branches => app.branches.len(),
        Tab::Log => 1 + app.log_entries.len(), // +1 for uncommitted item
        Tab::Stash => 0,
    }
}
```

- [ ] **Step 5: Update `enter_commit_detail` to handle index offset**

In `crates/gitat-ui/src/event/commit_detail.rs`, update the index lookup to subtract 1 for the uncommitted item:

```rust
pub(super) fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,  // offset: index 0 is uncommitted item
        _ => return,
    };
    let commit = match app.log_entries.get(idx) {
        Some(c) => c.clone(),
        None => return,
    };
    // ... rest unchanged
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p gitat-ui`
Expected: All tests PASS. Note: the existing test `test_enter_log_tab_enters_commit_detail_mode` in commit_detail.rs needs to be updated — it sets `log_list_state.select(Some(0))` but now index 0 is the uncommitted item. Update it to select index 1:

In `crates/gitat-ui/src/event/commit_detail.rs` test `test_enter_log_tab_enters_commit_detail_mode`, change:

```rust
app.log_list_state.select(Some(1)); // index 0 is uncommitted item, index 1 is first commit
```

Run: `cargo test -p gitat-ui`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/gitat-ui/src/event/normal.rs crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/uncommitted_detail.rs
git commit -m "feat: update Enter dispatch for uncommitted item and commit detail offset"
```

---

### Task 6: Remove Tab::Status, reorder tabs, update all references

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:16-50` (Tab enum)
- Modify: `crates/gitat-ui/src/app.rs:94-100` (App::new)
- Modify: `crates/gitat-ui/src/app.rs:121-128` (current_list_state_mut)
- Modify: `crates/gitat-ui/src/app.rs:164-183` (tests)
- Modify: `crates/gitat-ui/src/event/normal.rs:60-67` (s key)
- Modify: `crates/gitat-ui/src/event/normal.rs:130-137` (list_len)
- Modify: `crates/gitat-ui/src/event/staging.rs:94-96` (load_diff_for_selected Tab check)
- Modify: `crates/gitat-ui/src/views/mod.rs` (remove status module)
- Delete: `crates/gitat-ui/src/views/status.rs`
- Modify: `crates/gitat/src/main.rs:56-76` (tab bar)
- Modify: `crates/gitat-ui/src/event/mod.rs:125-182` (tests)
- Modify: `crates/gitat-ui/src/event/staging.rs:119-215` (tests)

- [ ] **Step 1: Update Tab enum in app.rs**

Replace the `Tab` enum and its impl:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Log,
    Branches,
    Stash,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Log => Tab::Branches,
            Tab::Branches => Tab::Stash,
            Tab::Stash => Tab::Log,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Log => Tab::Stash,
            Tab::Branches => Tab::Log,
            Tab::Stash => Tab::Branches,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Log => "Log",
            Tab::Branches => "Branches",
            Tab::Stash => "Stash",
        }
    }
}
```

- [ ] **Step 2: Update App::new() default tab and remove status_list_state**

In `App::new()`, change:

```rust
tab: Tab::Log,
```

Remove `status_list_state` field from the `App` struct and from `App::new()`. The `uncommitted_list_state` added in Task 1 replaces it.

Update `current_list_state_mut`:

```rust
pub fn current_list_state_mut(&mut self) -> &mut ListState {
    match self.tab {
        Tab::Branches => &mut self.branches_list_state,
        Tab::Log => &mut self.log_list_state,
        Tab::Stash => &mut self.log_list_state, // fallback
    }
}
```

- [ ] **Step 3: Update app.rs tests**

Replace the test module in `crates/gitat-ui/src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_cycling() {
        assert_eq!(Tab::Log.next(), Tab::Branches);
        assert_eq!(Tab::Stash.next(), Tab::Log);
        assert_eq!(Tab::Log.prev(), Tab::Stash);
        assert_eq!(Tab::Branches.prev(), Tab::Log);
    }

    #[test]
    fn test_app_initial_state() {
        let app = App::new();
        assert_eq!(app.tab, Tab::Log);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.panel, Panel::Left);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_uncommitted_detail_mode_exists() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        assert_eq!(app.mode, Mode::UncommittedDetail);
    }

    #[test]
    fn test_status_message_not_cleared_before_expiry() {
        let mut app = App::new();
        app.set_status_message("hello");
        app.clear_expired_status_message();
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_status_message_cleared_after_expiry() {
        let mut app = App::new();
        app.set_status_message("hello");
        app.status_message_set_at = Some(Instant::now() - Duration::from_secs(4));
        app.clear_expired_status_message();
        assert!(app.status_message.is_none());
        assert!(app.status_message_set_at.is_none());
    }
}
```

- [ ] **Step 4: Update normal.rs — remove Status-specific code**

Update the `s` key handler in `crates/gitat-ui/src/event/normal.rs`:

```rust
KeyCode::Char('s') => {
    if app.tab == Tab::Log && matches!(app.mode, Mode::Normal) {
        // s key is only valid inside UncommittedDetail mode, not in log list
    }
}
```

Actually, in Normal mode on the Log tab, `s` should do nothing (staging is only available inside UncommittedDetail mode, which has its own handler in Task 2). So simply remove the `s` handler from normal.rs:

```rust
KeyCode::Char('s') => {
    // Staging is handled in UncommittedDetail mode, not in Normal mode
}
```

Update `list_len` to remove `Tab::Status`:

```rust
fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::Branches => app.branches.len(),
        Tab::Log => 1 + app.log_entries.len(),
        Tab::Stash => 0,
    }
}
```

Remove the `Tab::Status` import if it was used (it's imported via `use crate::app::{App, Mode, Panel, Tab};` — `Tab` is still needed).

- [ ] **Step 5: Update staging.rs — remove Tab::Status reference and simplify helper**

In `crates/gitat-ui/src/event/staging.rs`, simplify `selected_status_index` since `status_list_state` is now removed:

```rust
fn selected_status_index(app: &App) -> Option<usize> {
    app.uncommitted_list_state.selected()
}
```

Update `load_diff_for_selected`:

```rust
pub(super) fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if !matches!(app.mode, Mode::UncommittedDetail) {
        return;
    }
    let idx = match selected_status_index(app) {
        Some(i) => i,
        None => return,
    };
    // ... rest unchanged
```

Remove `Tab` from the import since it's no longer used:

```rust
use crate::app::{App, Mode};
```

- [ ] **Step 6: Update views/mod.rs — remove status module**

Replace `crates/gitat-ui/src/views/mod.rs`:

```rust
pub mod branches;
pub mod commit;
pub mod log;

use ratatui::Frame;
use ratatui::layout::Rect;
use crate::app::{App, Tab};

pub fn render_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Log => log::render(f, app, area),
        Tab::Branches => branches::render(f, app, area),
        Tab::Stash => {} // Placeholder
    }
}
```

- [ ] **Step 7: Delete views/status.rs**

```bash
rm crates/gitat-ui/src/views/status.rs
```

- [ ] **Step 8: Update main.rs tab bar**

In `crates/gitat/src/main.rs`, replace the tab bar rendering:

```rust
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
```

- [ ] **Step 9: Update event/mod.rs tests**

In `crates/gitat-ui/src/event/mod.rs`, update tests that reference `Tab::Status`:

`test_tab_switch`:
```rust
#[test]
fn test_tab_switch() {
    let mut app = App::new();
    let runner = MockRunner::new();
    assert_eq!(app.tab, Tab::Log);
    handle_key(&mut app, mock_key(KeyCode::Tab), &runner);
    assert_eq!(app.tab, Tab::Branches);
}
```

- [ ] **Step 10: Update staging.rs tests**

In `crates/gitat-ui/src/event/staging.rs`, update all tests that reference `Tab::Status` to use `Mode::UncommittedDetail` instead:

`test_s_in_right_panel_calls_stage_hunk`:
```rust
#[test]
fn test_s_in_right_panel_calls_stage_hunk() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.mode = Mode::UncommittedDetail;
    app.panel = Panel::Right;
    // ... rest stays the same but use uncommitted_list_state:
    app.uncommitted_list_state.select(Some(0));
    // ... rest unchanged
```

`test_s_in_left_panel_still_stages_file`:
```rust
#[test]
fn test_s_in_left_panel_still_stages_file() {
    let mut app = App::new();
    app.tab = Tab::Log;
    app.mode = Mode::UncommittedDetail;
    app.panel = Panel::Left;
    // ... rest stays the same but use uncommitted_list_state:
    app.uncommitted_list_state.select(Some(0));
    // ... rest unchanged
```

Also update the imports in the staging tests module to include `Mode`:

```rust
use crate::app::{App, Mode, Panel, Tab};
```

- [ ] **Step 11: Update `handle_commit` in event/mod.rs to return to UncommittedDetail after commit**

In `crates/gitat-ui/src/event/mod.rs`, update the `Ok` branch in `handle_commit`:

```rust
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
            if app.return_to_uncommitted_detail {
                app.mode = Mode::UncommittedDetail;
                app.return_to_uncommitted_detail = false;
            } else {
                app.mode = Mode::Normal;
            }
            app.set_status_message("Committed successfully");
            app.refresh(runner);
        }
        Err(e) => {
            app.set_status_message(format!("Commit failed: {e}"));
            app.return_to_uncommitted_detail = false;
            app.mode = Mode::Normal;
        }
    }
}
```

Also reset the flag when pressing Esc in commit mode:

```rust
KeyCode::Esc => {
    if app.return_to_uncommitted_detail {
        app.mode = Mode::UncommittedDetail;
        app.return_to_uncommitted_detail = false;
    } else {
        app.mode = Mode::Normal;
    }
}
```

- [ ] **Step 12: Run full test suite**

Run: `cargo test`
Expected: All tests PASS, no compilation errors

- [ ] **Step 13: Commit**

```bash
git add -A
git commit -m "refactor: remove Status tab, make Log the default screen

Remove Tab::Status and views/status.rs. All status functionality is
now accessible via the Uncommitted Changes item in the Log view.
Tab order: Log | Branches | Stash. Commit from UncommittedDetail
returns to UncommittedDetail mode."
```

---

### Task 7: Final verification and cleanup

**Files:**
- All modified files from previous tasks

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings or errors

- [ ] **Step 3: Verify the binary compiles and runs**

Run: `cargo build`
Expected: Builds successfully

- [ ] **Step 4: Commit any cleanup if needed**

If clippy found issues, fix and commit:

```bash
git add -A
git commit -m "fix: address clippy warnings"
```
