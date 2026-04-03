# Commit Detail View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a commit detail view accessible from the Log tab that shows commit metadata, changed file list, and side-by-side diff.

**Architecture:** When Enter is pressed on a commit in the Log tab, fetch changed files via `git diff-tree`, switch to `Mode::CommitDetail`, and render a two-panel layout (file list + diff) with a metadata header. The data layer lives in `gitat-core/src/commit_detail.rs`, state management in `gitat-ui/src/app.rs`, event handling in `gitat-ui/src/event.rs`, and rendering in `gitat-ui/src/views/log.rs`.

**Tech Stack:** Rust, ratatui, crossterm, insta (snapshot tests)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/gitat-core/src/commit_detail.rs` | Create | `CommitFileEntry`, `FileChangeStatus`, `get_commit_files()`, `get_commit_file_diff()` |
| `crates/gitat-core/src/lib.rs` | Modify | Add `pub mod commit_detail;` |
| `crates/gitat-ui/src/app.rs` | Modify | Add `Mode::CommitDetail`, commit detail fields to `App`, initialization |
| `crates/gitat-ui/src/event.rs` | Modify | Add `handle_commit_detail()`, modify Enter in Log tab, add Mode dispatch |
| `crates/gitat-ui/src/views/log.rs` | Modify | Add commit detail rendering (metadata + file list + diff panels) |
| `crates/gitat-ui/src/theme.rs` | Modify | Add `file_added()` style |

---

### Task 1: Data Layer — Types and Parsing

**Files:**
- Create: `crates/gitat-core/src/commit_detail.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write tests for `parse_commit_files`**

In `crates/gitat-core/src/commit_detail.rs`:

```rust
use crate::GitError;
use crate::diff::{self, DiffFile};
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitFileEntry {
    pub path: String,
    pub status: FileChangeStatus,
}

pub fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, GitError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_commit_files("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_modified_file() {
        let input = "M\tsrc/main.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[0].status, FileChangeStatus::Modified);
    }

    #[test]
    fn test_parse_multiple_files() {
        let input = "A\tsrc/new.rs\nM\tsrc/main.rs\nD\tsrc/old.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].status, FileChangeStatus::Added);
        assert_eq!(result[0].path, "src/new.rs");
        assert_eq!(result[1].status, FileChangeStatus::Modified);
        assert_eq!(result[1].path, "src/main.rs");
        assert_eq!(result[2].status, FileChangeStatus::Deleted);
        assert_eq!(result[2].path, "src/old.rs");
    }

    #[test]
    fn test_parse_renamed_file() {
        let input = "R100\told_name.rs\tnew_name.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, FileChangeStatus::Renamed);
        // For renamed files, use the new path
        assert_eq!(result[0].path, "new_name.rs");
    }

    #[test]
    fn snapshot_multiple_files() {
        let input = "A\tsrc/new.rs\nM\tsrc/main.rs\nD\tsrc/old.rs\n";
        let result = parse_commit_files(input).unwrap();
        insta::assert_debug_snapshot!(result);
    }
}
```

- [ ] **Step 2: Add module to `lib.rs`**

In `crates/gitat-core/src/lib.rs`, add:

```rust
pub mod commit_detail;
```

Add it after the `pub mod commit;` line.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p gitat-core commit_detail`
Expected: FAIL — `todo!()` panics

- [ ] **Step 4: Implement `parse_commit_files`**

Replace the `todo!()` in `parse_commit_files`:

```rust
pub fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, GitError> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "<status>\t<path>" or for renames: "R<score>\t<old>\t<new>"
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let status_str = parts[0];
        let (status, path) = if status_str.starts_with('R') {
            // Rename: use the new path (parts[2] if available, else parts[1])
            let new_path = if parts.len() >= 3 { parts[2] } else { parts[1] };
            (FileChangeStatus::Renamed, new_path.to_string())
        } else {
            let status = match status_str {
                "A" => FileChangeStatus::Added,
                "D" => FileChangeStatus::Deleted,
                _ => FileChangeStatus::Modified,
            };
            (status, parts[1].to_string())
        };
        entries.push(CommitFileEntry { path, status });
    }
    Ok(entries)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p gitat-core commit_detail`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-core/src/commit_detail.rs crates/gitat-core/src/lib.rs
git commit -m "feat: add commit file list parsing in commit_detail module"
```

---

### Task 2: Data Layer — Git Commands

**Files:**
- Modify: `crates/gitat-core/src/commit_detail.rs`

- [ ] **Step 1: Write tests for `get_commit_files` and `get_commit_file_diff`**

Add these tests to the `mod tests` block in `commit_detail.rs`:

```rust
    #[test]
    fn test_get_commit_files() {
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\nA\tsrc/new.rs\n",
            );
        let result = get_commit_files(&runner, "abc123").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[1].path, "src/new.rs");
    }

    #[test]
    fn test_get_commit_file_diff_with_parent() {
        let diff_output = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    old();
+    new();
+    extra();
 }
";
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "diff parent123..abc123 -- src/main.rs",
                diff_output,
            );
        let result = get_commit_file_diff(&runner, "abc123", Some("parent123"), "src/main.rs").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "src/main.rs");
    }

    #[test]
    fn test_get_commit_file_diff_root_commit() {
        let diff_output = "\
diff --git a/src/main.rs b/src/main.rs
new file mode 100644
--- /dev/null
+++ b/src/main.rs
@@ -0,0 +1,2 @@
+fn main() {}
+fn helper() {}
";
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "show --format= abc123 -- src/main.rs",
                diff_output,
            );
        let result = get_commit_file_diff(&runner, "abc123", None, "src/main.rs").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "/dev/null");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-core commit_detail`
Expected: FAIL — functions not defined

- [ ] **Step 3: Implement `get_commit_files` and `get_commit_file_diff`**

Add these functions to `commit_detail.rs` (after `parse_commit_files`):

```rust
pub fn get_commit_files(
    runner: &dyn CommandRunner,
    hash: &str,
) -> Result<Vec<CommitFileEntry>, GitError> {
    let output = runner.run(&["diff-tree", "--no-commit-id", "-r", "--name-status", hash])?;
    parse_commit_files(&output)
}

pub fn get_commit_file_diff(
    runner: &dyn CommandRunner,
    hash: &str,
    parent_hash: Option<&str>,
    file_path: &str,
) -> Result<Vec<DiffFile>, GitError> {
    let output = if let Some(parent) = parent_hash {
        let range = format!("{parent}..{hash}");
        runner.run(&["diff", &range, "--", file_path])?
    } else {
        runner.run(&["show", "--format=", hash, "--", file_path])?
    };
    diff::parse_diff(&output)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-core commit_detail`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-core/src/commit_detail.rs
git commit -m "feat: add get_commit_files and get_commit_file_diff functions"
```

---

### Task 3: State Management — Mode and App Fields

**Files:**
- Modify: `crates/gitat-ui/src/app.rs`

- [ ] **Step 1: Add `Mode::CommitDetail` variant**

In `crates/gitat-ui/src/app.rs`, add `CommitDetail` to the `Mode` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Commit { message: String },
    Conflict { file: String },
    Search { query: String },
    Help,
    CommitDetail,
}
```

- [ ] **Step 2: Add commit detail fields to `App`**

Add these fields to the `App` struct (after `conflict_file`):

```rust
    pub commit_detail_commit: Option<CommitInfo>,
    pub commit_detail_files: Vec<gitat_core::commit_detail::CommitFileEntry>,
    pub commit_detail_file_state: ListState,
    pub commit_detail_panel: Panel,
    pub commit_detail_diff: Option<Vec<DiffFile>>,
    pub commit_detail_diff_state: SideBySideDiffState,
```

Add the import at the top of the file:

```rust
use gitat_core::commit_detail::CommitFileEntry;
```

Then update the field type to use the import:

```rust
    pub commit_detail_files: Vec<CommitFileEntry>,
```

- [ ] **Step 3: Initialize fields in `App::new()`**

Add these initializations in the `App::new()` method (after `conflict_file: None`):

```rust
            commit_detail_commit: None,
            commit_detail_files: Vec::new(),
            commit_detail_file_state: ListState::default(),
            commit_detail_panel: Panel::Left,
            commit_detail_diff: None,
            commit_detail_diff_state: SideBySideDiffState::new(),
```

- [ ] **Step 4: Run tests to verify compilation**

Run: `cargo test -p gitat-ui`
Expected: ALL PASS (existing tests still work)

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat: add Mode::CommitDetail and commit detail state fields to App"
```

---

### Task 4: Theme — File Change Status Colors

**Files:**
- Modify: `crates/gitat-ui/src/theme.rs`

- [ ] **Step 1: Add `file_added()` style**

In `crates/gitat-ui/src/theme.rs`, add a new method to `Theme` (after `file_untracked()`):

```rust
    pub fn file_added() -> Style { Style::new().fg(Color::Green) }
```

Note: `file_staged()` already returns green and can be reused for Added files, but a dedicated `file_added()` makes intent clearer. `file_unstaged()` (red) can be reused for Deleted. `commit_hash()` (yellow) can be reused for Modified.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p gitat-ui`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/gitat-ui/src/theme.rs
git commit -m "feat: add file_added theme style for commit detail view"
```

---

### Task 5: Event Handling — Enter in Log Tab and CommitDetail Mode

**Files:**
- Modify: `crates/gitat-ui/src/event.rs`

- [ ] **Step 1: Write tests for commit detail event handling**

Add these tests to `mod tests` in `event.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui event`
Expected: FAIL — `Mode::CommitDetail` arm not matched in `handle_key`

- [ ] **Step 3: Add `Mode::CommitDetail` dispatch in `handle_key`**

In `event.rs`, update the `handle_key` function to add the `CommitDetail` arm:

```rust
pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match app.mode {
        Mode::Normal => handle_normal(app, key, runner),
        Mode::Commit { .. } => handle_commit(app, key, runner),
        Mode::Help => handle_help(app, key),
        Mode::Search { .. } => handle_search(app, key),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => handle_commit_detail(app, key, runner),
    }
}
```

- [ ] **Step 4: Modify Enter handling in `handle_normal` for Log tab**

In `handle_normal`, replace the `KeyCode::Enter` arm:

```rust
        KeyCode::Enter => {
            if app.tab == Tab::Log {
                enter_commit_detail(app, runner);
            } else {
                load_diff_for_selected(app, runner);
            }
        }
```

- [ ] **Step 5: Implement `enter_commit_detail`**

Add this function after `load_diff_for_selected`:

```rust
fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
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

    app.commit_detail_commit = Some(commit.clone());
    app.commit_detail_files = files;
    app.commit_detail_file_state = ListState::default();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    app.commit_detail_panel = Panel::Left;
    app.commit_detail_diff_state = SideBySideDiffState::new();
    app.mode = Mode::CommitDetail;
}
```

- [ ] **Step 6: Implement `load_commit_detail_diff`**

Add this helper function:

```rust
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
```

- [ ] **Step 7: Implement `handle_commit_detail`**

Add this function:

```rust
fn handle_commit_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.commit_detail_commit = None;
            app.commit_detail_files.clear();
            app.commit_detail_file_state = ListState::default();
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
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p gitat-ui event`
Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add crates/gitat-ui/src/event.rs
git commit -m "feat: add commit detail event handling and Enter key in Log tab"
```

---

### Task 6: Rendering — Commit Detail View

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Add imports**

Update imports at the top of `crates/gitat-ui/src/views/log.rs`:

```rust
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use gitat_core::commit_detail::FileChangeStatus;
use crate::app::{App, Mode, Panel};
use crate::theme::Theme;
use crate::widgets::side_by_side_diff::SideBySideDiff;
```

- [ ] **Step 2: Update `render` to branch on mode**

Replace the `render` function:

```rust
pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if matches!(app.mode, Mode::CommitDetail) {
        render_commit_detail(f, app, area);
    } else {
        render_log_list(f, app, area);
    }
}
```

- [ ] **Step 3: Extract existing log list rendering**

Rename the existing rendering logic into `render_log_list`:

```rust
fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.log_entries.iter().map(|c| {
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
        ListItem::new(Line::from(spans))
    }).collect();

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

- [ ] **Step 4: Implement `render_commit_detail`**

Add the commit detail rendering function:

```rust
fn render_commit_detail(f: &mut Frame, app: &mut App, area: Rect) {
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

    render_file_list(f, app, panels[0]);
    render_commit_diff(f, app, panels[1]);
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.commit_detail_panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

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
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.commit_detail_file_state);
}

fn render_commit_diff(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.commit_detail_panel == Panel::Right;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.commit_detail_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let widget = SideBySideDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.commit_detail_diff_state);
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

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: OK

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-ui/src/views/log.rs
git commit -m "feat: add commit detail view rendering with metadata, file list, and diff panels"
```

---

### Task 7: Integration — Verify Full Build and Run Tests

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: ALL PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: No errors

- [ ] **Step 3: Update insta snapshots if needed**

Run: `cargo insta test --workspace`
Then: `cargo insta accept` if new snapshots were created

- [ ] **Step 4: Final commit if snapshot updates**

```bash
git add -A
git commit -m "test: update insta snapshots for commit detail view"
```
