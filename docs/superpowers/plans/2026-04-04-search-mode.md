# Search Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement incremental filtering search mode (`/`) for the log list, allowing users to filter commits by message, hash, or author in real-time.

**Architecture:** Add `filtered_log_indices` and `pre_search_cursor` fields to `App`. Filter logic runs on each keystroke in search mode, updating the index list. The log view conditionally renders filtered entries. The status bar doubles as the search input display.

**Tech Stack:** Rust, ratatui, crossterm

---

### Task 1: Add Search Fields to App

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:65-122`

- [ ] **Step 1: Write the test for new fields**

Add to the existing `mod tests` block in `app.rs`:

```rust
#[test]
fn test_app_search_fields_initial_state() {
    let app = App::new();
    assert_eq!(app.pre_search_cursor, None);
    assert_eq!(app.filtered_log_indices, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_app_search_fields_initial_state`
Expected: FAIL — fields don't exist yet.

- [ ] **Step 3: Add fields to App struct and new()**

In `app.rs`, add two fields after `uncommitted_file_map`:

```rust
pub struct App {
    // ... existing fields ...
    pub uncommitted_file_map: Vec<Option<(usize, bool)>>,
    /// Original cursor position before entering search mode, for Esc restoration.
    pub pre_search_cursor: Option<usize>,
    /// Indices into `log_entries` matching the current search query.
    /// `None` = no filter (normal display). `Some(vec)` = filtered view.
    pub filtered_log_indices: Option<Vec<usize>>,
}
```

In `App::new()`, add:

```rust
pre_search_cursor: None,
filtered_log_indices: None,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_app_search_fields_initial_state`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat(search): add pre_search_cursor and filtered_log_indices fields to App"
```

---

### Task 2: Implement Filter Logic

**Files:**
- Modify: `crates/gitat-ui/src/app.rs`

- [ ] **Step 1: Write the test for filter logic**

Add to `mod tests` in `app.rs`:

```rust
#[test]
fn test_update_search_filter_matches_message() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "fix login bug".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "add tests".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "ccc".into(),
            short_hash: "ccc".into(),
            author: "Alice".into(),
            date: "2026-01-03".into(),
            message: "update readme".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.log_list_state.select(Some(2));

    // Filter by "fix" — only first entry matches
    app.update_search_filter("fix");
    assert_eq!(app.filtered_log_indices, Some(vec![0]));
    assert_eq!(app.log_list_state.selected(), Some(0));

    // Filter by "alice" (case-insensitive) — matches author in entries 0 and 2
    app.update_search_filter("alice");
    assert_eq!(app.filtered_log_indices, Some(vec![0, 2]));

    // Filter by "bbb" — matches short_hash
    app.update_search_filter("bbb");
    assert_eq!(app.filtered_log_indices, Some(vec![1]));

    // Empty query — clears filter
    app.update_search_filter("");
    assert_eq!(app.filtered_log_indices, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_update_search_filter_matches_message`
Expected: FAIL — method doesn't exist.

- [ ] **Step 3: Implement update_search_filter**

Add to `impl App` in `app.rs`:

```rust
pub fn update_search_filter(&mut self, query: &str) {
    if query.is_empty() {
        self.filtered_log_indices = None;
        return;
    }
    let query_lower = query.to_lowercase();
    let indices: Vec<usize> = self
        .log_entries
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.message.to_lowercase().contains(&query_lower)
                || c.short_hash.to_lowercase().contains(&query_lower)
                || c.author.to_lowercase().contains(&query_lower)
        })
        .map(|(i, _)| i)
        .collect();
    self.filtered_log_indices = Some(indices);
    self.log_list_state.select(Some(0));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_update_search_filter_matches_message`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/app.rs
git commit -m "feat(search): implement update_search_filter method on App"
```

---

### Task 3: Save Cursor Position on `/` Key

**Files:**
- Modify: `crates/gitat-ui/src/event/normal.rs:108-112`

- [ ] **Step 1: Write the test**

Add to `mod tests` in `normal.rs`:

```rust
#[test]
fn test_slash_saves_cursor_and_enters_search() {
    let mut app = App::new();
    app.log_list_state.select(Some(3));
    let runner = MockRunner::new();
    handle_key(&mut app, mock_key(KeyCode::Char('/')), &runner);
    assert!(matches!(app.mode, Mode::Search { ref query } if query.is_empty()));
    assert_eq!(app.pre_search_cursor, Some(3));
}
```

Add `use crate::app::Mode;` to the test imports if not already present.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_slash_saves_cursor_and_enters_search`
Expected: FAIL — `pre_search_cursor` is not set to `Some(3)`.

- [ ] **Step 3: Update the `/` handler**

In `normal.rs`, change the `/` handler (lines 108-112):

```rust
KeyCode::Char('/') => {
    app.pre_search_cursor = app.log_list_state.selected();
    app.mode = Mode::Search {
        query: String::new(),
    };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_slash_saves_cursor_and_enters_search`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/event/normal.rs
git commit -m "feat(search): save cursor position when entering search mode"
```

---

### Task 4: Update handle_search with Filter, Enter, and Esc

**Files:**
- Modify: `crates/gitat-ui/src/event/mod.rs:91-108`

- [ ] **Step 1: Write tests for search key handling**

Add to `mod tests` in `event/mod.rs`:

```rust
#[test]
fn test_search_typing_filters_log() {
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
            message: "add feature".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.mode = Mode::Search { query: String::new() };
    let runner = MockRunner::new();

    // Type 'f' — both match ("fix" and "feature")
    handle_key(&mut app, mock_key(KeyCode::Char('f')), &runner);
    assert!(matches!(&app.mode, Mode::Search { query } if query == "f"));
    assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));

    // Type 'i' — only "fix bug" matches
    handle_key(&mut app, mock_key(KeyCode::Char('i')), &runner);
    assert!(matches!(&app.mode, Mode::Search { query } if query == "fi"));
    assert_eq!(app.filtered_log_indices, Some(vec![0]));

    // Backspace — back to "f", both match again
    handle_key(&mut app, mock_key(KeyCode::Backspace), &runner);
    assert!(matches!(&app.mode, Mode::Search { query } if query == "f"));
    assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));
}

#[test]
fn test_search_enter_confirms_selection() {
    let mut app = App::new();
    app.log_entries = vec![
        gitat_core::log::CommitInfo {
            hash: "aaa".into(),
            short_hash: "aaa".into(),
            author: "Alice".into(),
            date: "2026-01-01".into(),
            message: "first".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "bbb".into(),
            short_hash: "bbb".into(),
            author: "Bob".into(),
            date: "2026-01-02".into(),
            message: "second".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
        gitat_core::log::CommitInfo {
            hash: "ccc".into(),
            short_hash: "ccc".into(),
            author: "Charlie".into(),
            date: "2026-01-03".into(),
            message: "third".into(),
            refs: vec![],
            parent_hashes: vec![],
        },
    ];
    app.pre_search_cursor = Some(0);
    app.mode = Mode::Search { query: "second".into() };
    app.update_search_filter("second");
    // filtered_log_indices = Some([1]), selection = 0 (first filtered item)
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.filtered_log_indices, None);
    // Original index 1 maps to log_list_state index 2 (offset by 1 for uncommitted row)
    assert_eq!(app.log_list_state.selected(), Some(2));
}

#[test]
fn test_search_esc_restores_cursor() {
    let mut app = App::new();
    app.pre_search_cursor = Some(5);
    app.mode = Mode::Search { query: "test".into() };
    app.filtered_log_indices = Some(vec![0, 2]);
    let runner = MockRunner::new();

    handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.filtered_log_indices, None);
    assert_eq!(app.log_list_state.selected(), Some(5));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gitat-ui test_search_typing_filters_log test_search_enter_confirms_selection test_search_esc_restores_cursor`
Expected: FAIL — current handle_search doesn't filter or handle Enter.

- [ ] **Step 3: Rewrite handle_search**

Replace the `handle_search` function in `event/mod.rs`:

```rust
fn handle_search(app: &mut App, key: KeyEvent) {
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
            let original_index = app
                .log_list_state
                .selected()
                .and_then(|sel| {
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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gitat-ui test_search_typing_filters_log test_search_enter_confirms_selection test_search_esc_restores_cursor`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/event/mod.rs
git commit -m "feat(search): implement filter, Enter confirm, and Esc cancel in search mode"
```

---

### Task 5: Update Log View Rendering for Filtered Results

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs:35-128`

- [ ] **Step 1: Update render_log_list to include search bar area**

In `render_log_list`, add a conditional layout split for the search bar:

```rust
fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_searching = matches!(app.mode, Mode::Search { .. });

    let outer_chunks = if is_searching {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    let main_area = outer_chunks[0];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_area);

    render_log_list_items(f, app, chunks[0]);

    match app.log_list_state.selected() {
        Some(0) if app.filtered_log_indices.is_none() => {
            render_uncommitted_preview(f, app, chunks[1]);
        }
        Some(_) => render_commit_preview(f, app, chunks[1]),
        None => {}
    }

    // Search bar
    if is_searching {
        if let Mode::Search { ref query } = app.mode {
            let search_text = format!("/{query}_");
            let search_bar = Paragraph::new(search_text).style(Theme::status_bar());
            f.render_widget(search_bar, outer_chunks[1]);
        }
    }
}
```

- [ ] **Step 2: Update render_log_list_items for filtered mode**

Replace `render_log_list_items` with filtered-aware rendering:

```rust
fn render_log_list_items(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = if let Some(ref indices) = app.filtered_log_indices {
        // Filtered mode: show only matching commits, no graph, no uncommitted row
        indices
            .iter()
            .map(|&i| {
                let c = &app.log_entries[i];
                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled(&c.short_hash, Theme::commit_hash()));
                spans.push(Span::raw(" "));
                if !c.refs.is_empty() {
                    spans.push(Span::styled(
                        format!("({}) ", c.refs.join(", ")),
                        Theme::commit_ref(),
                    ));
                }
                spans.push(Span::raw(&c.message));
                ListItem::new(Line::from(spans))
            })
            .collect()
    } else {
        // Normal mode: full rendering with graph and uncommitted row
        let mut items: Vec<ListItem> = Vec::new();
        let graph_rows = graph::build_graph(&app.log_entries);

        // Uncommitted changes item (always at index 0)
        let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
        let unstaged_count = app
            .status
            .iter()
            .filter(|e| !e.is_staged() && e.worktree_status != FileStatus::Untracked)
            .count();
        let untracked_count = app
            .status
            .iter()
            .filter(|e| e.index_status == FileStatus::Untracked)
            .count();
        let total_changes = staged_count + unstaged_count + untracked_count;

        let mut uncommitted_spans: Vec<Span> = Vec::new();
        if let Some(first_row) = graph_rows.first() {
            if total_changes > 0 {
                uncommitted_spans.push(Span::styled("* ", Theme::graph_color(0)));
                for cell in first_row.cells.iter().skip(1) {
                    if cell.symbol != ' ' {
                        uncommitted_spans
                            .push(Span::styled("| ", Theme::graph_color(cell.color_index)));
                    } else {
                        uncommitted_spans.push(Span::raw("  "));
                    }
                }
            } else {
                for cell in &first_row.cells {
                    if cell.symbol != ' ' {
                        uncommitted_spans
                            .push(Span::styled("| ", Theme::graph_color(cell.color_index)));
                    } else {
                        uncommitted_spans.push(Span::raw("  "));
                    }
                }
            }
        } else if total_changes > 0 {
            uncommitted_spans.push(Span::styled("* ", Theme::graph_color(0)));
        }

        if total_changes > 0 {
            uncommitted_spans.extend([
                Span::styled("● ", Theme::border_focused()),
                Span::styled("Uncommitted Changes", Theme::border_focused()),
                Span::styled(
                    format!(
                        " — {} staged, {} unstaged",
                        staged_count,
                        unstaged_count + untracked_count
                    ),
                    Theme::diff_context(),
                ),
            ]);
        } else {
            uncommitted_spans.extend([
                Span::styled("● ", Theme::border_focused()),
                Span::styled("Uncommitted Changes", Theme::border_focused()),
            ]);
        }
        items.push(ListItem::new(Line::from(uncommitted_spans)));

        for (i, c) in app.log_entries.iter().enumerate() {
            let mut spans: Vec<Span> = Vec::new();
            if let Some(row) = graph_rows.get(i) {
                for cell in &row.cells {
                    let s = format!("{} ", cell.symbol);
                    spans.push(Span::styled(s, Theme::graph_color(cell.color_index)));
                }
            }
            spans.push(Span::styled(&c.short_hash, Theme::commit_hash()));
            spans.push(Span::raw(" "));
            if !c.refs.is_empty() {
                spans.push(Span::styled(
                    format!("({}) ", c.refs.join(", ")),
                    Theme::commit_ref(),
                ));
            }
            spans.push(Span::raw(&c.message));
            items.push(ListItem::new(Line::from(spans)));
        }

        items
    };

    let title = if app.filtered_log_indices.is_some() {
        " Log (filtered) "
    } else {
        " Log "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.log_list_state);
}
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p gitat-ui`
Expected: All existing tests + new tests pass.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy`
Expected: No warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/src/views/log.rs
git commit -m "feat(search): render filtered log list and inline search bar"
```

---

### Task 6: Update Preview Loading for Filtered Mode

**Files:**
- Modify: `crates/gitat-ui/src/event/mod.rs:23-29`

- [ ] **Step 1: Update load_log_preview to handle filtered mode**

The current `load_log_preview` assumes `selected() == 0` means uncommitted changes. In filtered mode, there is no uncommitted row. Update:

```rust
pub fn load_log_preview(app: &mut App, runner: &dyn CommandRunner) {
    if app.filtered_log_indices.is_some() {
        // In filtered mode, all items are commits (no uncommitted row)
        if app.log_list_state.selected().is_some() {
            commit_detail::load_commit_preview(app, runner);
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

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p gitat-ui`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/gitat-ui/src/event/mod.rs
git commit -m "feat(search): handle filtered mode in load_log_preview"
```

---

### Task 7: Final Integration Testing and Cleanup

**Files:**
- All modified files

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test`
Expected: All tests pass across all crates.

- [ ] **Step 2: Run clippy and fmt**

Run: `cargo clippy && cargo fmt --check`
Expected: No warnings, no formatting issues.

- [ ] **Step 3: Manual verification checklist**

Verify with `cargo run` in a git repository:
1. Press `/` — search bar appears at bottom, cursor saved
2. Type a query — log list filters incrementally
3. Press Enter — returns to Normal mode with selected commit focused
4. Press `/`, type, then Esc — returns to original cursor position
5. Empty query shows all entries

- [ ] **Step 4: Update TODO.md**

Mark the search mode item as completed in `TODO.md`:

```markdown
- [x] 検索モード (`/`) — `Mode::Search` に入るがフィルタリングロジックが未実装
```

- [ ] **Step 5: Commit**

```bash
git add TODO.md
git commit -m "docs: mark search mode as completed in TODO.md"
```
