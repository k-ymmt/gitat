# gitat-ui Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose the monolithic App struct into domain sub-structures, eliminate duplicated code, and consolidate rendering logic.

**Architecture:** Top-down approach — Phase 1 splits App into sub-structures (CommitDetailState, UncommittedState, SearchState, ConflictResolveState, StatusBar), Phase 2 adds status filtering helpers, Phase 3 consolidates rendering, Phase 4 polishes constants and dead code.

**Tech Stack:** Rust, ratatui, crossterm, gitat-core

**Spec:** `docs/superpowers/specs/2026-04-05-refactoring-design.md`

---

## File Structure

All changes are to existing files — no new files created.

**Modified files:**
- `crates/gitat-ui/src/app.rs` — Define sub-structures, refactor App fields and methods
- `crates/gitat-ui/src/event/mod.rs` — Update field access paths
- `crates/gitat-ui/src/event/normal.rs` — Update field access paths
- `crates/gitat-ui/src/event/commit_detail.rs` — Update field access paths
- `crates/gitat-ui/src/event/uncommitted_detail.rs` — Update field access paths
- `crates/gitat-ui/src/event/staging.rs` — Update field access paths
- `crates/gitat-ui/src/views/log.rs` — Update field access paths, add shared render helpers
- `crates/gitat-ui/src/views/branches.rs` — No changes needed (uses `app.branches` and `app.branches_list_state` which stay on App)
- `crates/gitat-ui/src/views/commit.rs` — No changes needed (only uses `app.mode`)
- `crates/gitat/src/main.rs` — Update field access paths

---

## Phase 1: App Struct Decomposition

### Task 1: Define sub-structures and refactor App struct

**Files:**
- Modify: `crates/gitat-ui/src/app.rs`

This task ONLY modifies app.rs. The code will not compile until Tasks 2-6 update all callers.

- [ ] **Step 1: Add sub-structure definitions above the App struct**

Add these structs after the `Panel` enum (after line 63) in `crates/gitat-ui/src/app.rs`:

```rust
pub struct CommitDetailState {
    pub commit: Option<CommitInfo>,
    pub files: Vec<CommitFileEntry>,
    pub file_state: ListState,
    pub panel: Panel,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
}

impl CommitDetailState {
    pub fn new() -> Self {
        Self {
            commit: None,
            files: Vec::new(),
            file_state: ListState::default(),
            panel: Panel::Left,
            diff: None,
            diff_state: UnifiedDiffState::new(),
        }
    }
}

impl Default for CommitDetailState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UncommittedState {
    pub list_state: ListState,
    /// Maps visual list index to (status_index, is_in_staged_section).
    /// None for section headers.
    pub file_map: Vec<Option<(usize, bool)>>,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
    pub panel: Panel,
}

impl UncommittedState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            file_map: Vec::new(),
            diff: None,
            diff_state: UnifiedDiffState::new(),
            panel: Panel::Left,
        }
    }
}

impl Default for UncommittedState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SearchState {
    /// Original cursor position before entering search mode, for Esc restoration.
    pub pre_search_cursor: Option<usize>,
    /// Indices into `log_entries` matching the current search query.
    /// `None` = no filter (normal display). `Some(vec)` = filtered view.
    pub filtered_log_indices: Option<Vec<usize>>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            pre_search_cursor: None,
            filtered_log_indices: None,
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConflictResolveState {
    pub editor_state: Option<ConflictEditorState>,
    pub file: Option<ConflictFile>,
}

impl ConflictResolveState {
    pub fn new() -> Self {
        Self {
            editor_state: None,
            file: None,
        }
    }
}

impl Default for ConflictResolveState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StatusBar {
    pub message: Option<String>,
    pub set_at: Option<Instant>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            message: None,
            set_at: None,
        }
    }

    pub fn set(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
        self.set_at = Some(Instant::now());
    }

    pub fn clear_if_expired(&mut self) {
        if let Some(set_at) = self.set_at
            && set_at.elapsed() > STATUS_MESSAGE_TIMEOUT
        {
            self.message = None;
            self.set_at = None;
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(3);
```

- [ ] **Step 2: Replace App struct fields with sub-structures**

Replace the entire `App` struct definition and `App::new()`:

```rust
pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub mode_stack: Vec<Mode>,
    pub should_quit: bool,
    pub return_to_uncommitted_detail: bool,
    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log_entries: Vec<CommitInfo>,
    pub log_list_state: ListState,
    pub branches_list_state: ListState,
    pub uncommitted: UncommittedState,
    pub commit_detail: CommitDetailState,
    pub search: SearchState,
    pub conflict: ConflictResolveState,
    pub status_bar: StatusBar,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: Tab::Log,
            mode: Mode::Normal,
            mode_stack: Vec::new(),
            should_quit: false,
            return_to_uncommitted_detail: false,
            status: Vec::new(),
            branches: Vec::new(),
            log_entries: Vec::new(),
            log_list_state: ListState::default(),
            branches_list_state: ListState::default(),
            uncommitted: UncommittedState::new(),
            commit_detail: CommitDetailState::new(),
            search: SearchState::new(),
            conflict: ConflictResolveState::new(),
            status_bar: StatusBar::new(),
        }
    }
```

- [ ] **Step 3: Update App methods to use sub-structures**

Update the remaining methods in `impl App`. Replace the old `set_status_message`, `clear_expired_status_message`, `rebuild_uncommitted_file_map`, `clamp_uncommitted_selection`, and `update_search_filter` methods:

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

    pub fn current_list_state_mut(&mut self) -> &mut ListState {
        match self.tab {
            Tab::Branches => &mut self.branches_list_state,
            Tab::Log => &mut self.log_list_state,
            Tab::Stash => &mut self.log_list_state, // fallback
        }
    }

    pub fn refresh(&mut self, runner: &dyn CommandRunner) {
        if let Ok(status) = gitat_core::status::get_status(runner) {
            self.status = status;
        }
        if let Ok(branches) = gitat_core::branch::list_branches(runner) {
            self.branches = branches;
        }
        if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
            self.log_entries = log;
        }
    }

    pub fn refresh_status_and_log(&mut self, runner: &dyn CommandRunner) {
        if let Ok(status) = gitat_core::status::get_status(runner) {
            self.status = status;
        }
        if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
            self.log_entries = log;
        }
    }

    pub fn rebuild_uncommitted_file_map(&mut self) {
        let mut map: Vec<Option<(usize, bool)>> = Vec::new();

        let staged: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
            })
            .map(|(i, _)| i)
            .collect();

        if !staged.is_empty() {
            map.push(None);
            for idx in staged {
                map.push(Some((idx, true)));
            }
        }

        let modified: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.worktree_status != FileStatus::Unmodified
                    && e.worktree_status != FileStatus::Untracked
            })
            .map(|(i, _)| i)
            .collect();

        if !modified.is_empty() {
            map.push(None);
            for idx in modified {
                map.push(Some((idx, false)));
            }
        }

        let untracked: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| e.index_status == FileStatus::Untracked)
            .map(|(i, _)| i)
            .collect();

        if !untracked.is_empty() {
            map.push(None);
            for idx in untracked {
                map.push(Some((idx, false)));
            }
        }

        self.uncommitted.file_map = map;
    }

    pub fn clamp_uncommitted_selection(&mut self) {
        if self.uncommitted.file_map.is_empty() {
            self.uncommitted.list_state.select(None);
            return;
        }

        let current = match self.uncommitted.list_state.selected() {
            Some(i) => i,
            None => {
                if let Some(pos) = self.uncommitted.file_map.iter().position(|x| x.is_some()) {
                    self.uncommitted.list_state.select(Some(pos));
                }
                return;
            }
        };

        if current < self.uncommitted.file_map.len()
            && self.uncommitted.file_map[current].is_some()
        {
            return;
        }

        let forward = self
            .uncommitted
            .file_map
            .iter()
            .enumerate()
            .skip(current)
            .find(|(_, x)| x.is_some())
            .map(|(i, _)| i);
        let backward = self.uncommitted.file_map
            [..current.min(self.uncommitted.file_map.len())]
            .iter()
            .rposition(|x| x.is_some());

        self.uncommitted.list_state.select(forward.or(backward));
    }

    pub fn update_search_filter(&mut self, query: &str) {
        if query.is_empty() {
            self.search.filtered_log_indices = None;
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
        self.search.filtered_log_indices = Some(indices);
        self.log_list_state.select(Some(0));
    }
}
```

Remove the old `set_status_message` and `clear_expired_status_message` methods entirely.

- [ ] **Step 4: Update tests in app.rs**

Replace the test module with updated field access paths:

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
        assert_eq!(app.uncommitted.panel, Panel::Left);
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
        app.status_bar.set("hello");
        app.status_bar.clear_if_expired();
        assert!(app.status_bar.message.is_some());
    }

    #[test]
    fn test_status_message_cleared_after_expiry() {
        let mut app = App::new();
        app.status_bar.set("hello");
        app.status_bar.set_at = Some(Instant::now() - Duration::from_secs(4));
        app.status_bar.clear_if_expired();
        assert!(app.status_bar.message.is_none());
        assert!(app.status_bar.set_at.is_none());
    }

    #[test]
    fn test_app_search_fields_initial_state() {
        let app = App::new();
        assert_eq!(app.search.pre_search_cursor, None);
        assert_eq!(app.search.filtered_log_indices, None);
    }

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

        app.update_search_filter("fix");
        assert_eq!(app.search.filtered_log_indices, Some(vec![0]));
        assert_eq!(app.log_list_state.selected(), Some(0));

        app.update_search_filter("alice");
        assert_eq!(app.search.filtered_log_indices, Some(vec![0, 2]));

        app.update_search_filter("bbb");
        assert_eq!(app.search.filtered_log_indices, Some(vec![1]));

        app.update_search_filter("");
        assert_eq!(app.search.filtered_log_indices, None);
    }

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
        assert!(app.branches.is_empty());
    }
}
```

### Task 2: Update event/commit_detail.rs

**Files:**
- Modify: `crates/gitat-ui/src/event/commit_detail.rs`

- [ ] **Step 1: Update field accesses**

Apply these replacements throughout the file:

| Old | New |
|-----|-----|
| `app.commit_detail_files` | `app.commit_detail.files` |
| `app.commit_detail_commit` | `app.commit_detail.commit` |
| `app.commit_detail_file_state` | `app.commit_detail.file_state` |
| `app.commit_detail_panel` | `app.commit_detail.panel` |
| `app.commit_detail_diff` | `app.commit_detail.diff` |
| `app.commit_detail_diff_state` | `app.commit_detail.diff_state` |
| `app.set_status_message(` | `app.status_bar.set(` |

Remove the import of `UnifiedDiffState` (it's now initialized via `CommitDetailState` methods or `UnifiedDiffState::new()` — but actually it's still used directly in `prepare_commit_detail` and `handle_commit_detail`, so keep the import).

The full updated file should be:

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::unified_diff::UnifiedDiffState;
use gitat_core::runner::CommandRunner;

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

    app.commit_detail.files = files;
    app.commit_detail.commit = Some(commit);
}

pub(super) fn load_commit_preview(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    load_commit_preview_at(app, runner, idx);
}

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
                app.status_bar.set(format!("Failed to load commit files: {e}"));
                return false;
            }
        };

    app.commit_detail.commit = Some(commit);
    app.commit_detail.files = files;
    app.commit_detail.file_state = ratatui::widgets::ListState::default();
    app.commit_detail.panel = Panel::Left;
    app.commit_detail.diff_state = UnifiedDiffState::new();
    if !app.commit_detail.files.is_empty() {
        app.commit_detail.file_state.select(Some(0));
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

fn load_commit_detail_diff(app: &mut App, runner: &dyn CommandRunner) {
    let commit = match &app.commit_detail.commit {
        Some(c) => c,
        None => return,
    };
    let idx = match app.commit_detail.file_state.selected() {
        Some(i) => i,
        None => return,
    };
    let file_entry = match app.commit_detail.files.get(idx) {
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
            app.commit_detail.diff = Some(diff);
            app.commit_detail.diff_state = UnifiedDiffState::new();
        }
        Err(e) => {
            app.status_bar.set(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_commit_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.pop_mode();
            if matches!(app.mode, Mode::CommitDetail) {
                app.mode = Mode::Normal;
            }
            app.commit_detail.file_state = ratatui::widgets::ListState::default();
            app.commit_detail.diff_state = UnifiedDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.commit_detail.panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.commit_detail.panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.commit_detail.panel == Panel::Left {
                let len = app.commit_detail.files.len();
                if len > 0 {
                    let i = match app.commit_detail.file_state.selected() {
                        Some(i) => (i + 1).min(len - 1),
                        None => 0,
                    };
                    app.commit_detail.file_state.select(Some(i));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail.diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.commit_detail.panel == Panel::Left {
                if let Some(i) = app.commit_detail.file_state.selected() {
                    let next = if i == 0 { 0 } else { i - 1 };
                    app.commit_detail.file_state.select(Some(next));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail.diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('J') => {
            app.commit_detail.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.commit_detail.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.commit_detail.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.commit_detail.diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.commit_detail.diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.commit_detail.diff_state.prev_hunk();
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Update tests in commit_detail.rs**

Replace test field accesses:

| Old | New |
|-----|-----|
| `app.commit_detail_commit` | `app.commit_detail.commit` |
| `app.commit_detail_files` | `app.commit_detail.files` |
| `app.commit_detail_panel` | `app.commit_detail.panel` |
| `app.filtered_log_indices` | `app.search.filtered_log_indices` |

The test module stays the same structure — just update the field paths. For example in `test_load_commit_preview_filtered_mode_index_zero`:

```rust
        app.search.filtered_log_indices = Some(vec![1]);
```

and:

```rust
        assert!(app.commit_detail.commit.is_some());
        assert_eq!(app.commit_detail.commit.as_ref().unwrap().hash, "bbb222");
        assert_eq!(app.commit_detail.files.len(), 1);
```

In `test_load_commit_preview_populates_data`:

```rust
        assert!(app.commit_detail.commit.is_some());
        assert_eq!(app.commit_detail.files.len(), 1);
        assert_eq!(app.commit_detail.files[0].path, "src/main.rs");
```

In `test_enter_log_tab_enters_commit_detail_mode`:

```rust
        assert!(app.commit_detail.commit.is_some());
        assert_eq!(app.commit_detail.files.len(), 1);
```

In `test_esc_from_commit_detail_returns_to_search`:

```rust
        app.search.filtered_log_indices = Some(vec![0, 2]);
        app.search.pre_search_cursor = Some(5);
        // ...
        assert_eq!(app.search.filtered_log_indices, Some(vec![0, 2]));
        assert_eq!(app.search.pre_search_cursor, Some(5));
```

In `test_esc_from_commit_detail_preserves_preview_data`:

```rust
        app.commit_detail.commit = Some(gitat_core::log::CommitInfo { ... });
        app.commit_detail.files = vec![...];
        // ...
        assert!(app.commit_detail.commit.is_some());
        assert_eq!(app.commit_detail.files.len(), 1);
```

In `test_commit_detail_panel_switch`:

```rust
        app.commit_detail.panel = Panel::Left;
        // ...
        assert_eq!(app.commit_detail.panel, Panel::Right);
        // ...
        assert_eq!(app.commit_detail.panel, Panel::Left);
```

### Task 3: Update event/uncommitted_detail.rs and event/staging.rs

**Files:**
- Modify: `crates/gitat-ui/src/event/uncommitted_detail.rs`
- Modify: `crates/gitat-ui/src/event/staging.rs`

- [ ] **Step 1: Update uncommitted_detail.rs field accesses**

Apply these replacements throughout `crates/gitat-ui/src/event/uncommitted_detail.rs`:

| Old | New |
|-----|-----|
| `app.uncommitted_list_state` | `app.uncommitted.list_state` |
| `app.uncommitted_file_map` | `app.uncommitted.file_map` |
| `app.current_diff` | `app.uncommitted.diff` |
| `app.diff_state` | `app.uncommitted.diff_state` |
| `app.panel` | `app.uncommitted.panel` |
| `app.set_status_message(` | `app.status_bar.set(` |

The full updated file:

```rust
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::unified_diff::UnifiedDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn load_uncommitted_preview(_app: &mut App, _runner: &dyn CommandRunner) {
    // Preview only shows file list from app.status (already loaded by refresh).
    // No additional data loading needed.
}

pub(super) fn enter_uncommitted_detail(app: &mut App, runner: &dyn CommandRunner) {
    app.uncommitted.list_state = ratatui::widgets::ListState::default();
    app.uncommitted.panel = Panel::Left;
    app.uncommitted.diff_state = UnifiedDiffState::new();
    app.uncommitted.diff = None;
    app.rebuild_uncommitted_file_map();

    // Select the first file entry (skip section headers)
    if let Some(first) = app.uncommitted.file_map.iter().position(|x| x.is_some()) {
        app.uncommitted.list_state.select(Some(first));
        load_uncommitted_diff(app, runner);
    }
    app.mode = Mode::UncommittedDetail;
}

fn load_uncommitted_diff(app: &mut App, runner: &dyn CommandRunner) {
    let visual_idx = match app.uncommitted.list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let (status_idx, staged) = match app.uncommitted.file_map.get(visual_idx) {
        Some(Some(info)) => *info,
        _ => return,
    };
    let entry = match app.status.get(status_idx) {
        Some(e) => e.clone(),
        None => return,
    };
    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.uncommitted.diff = Some(diff);
            app.uncommitted.diff_state = UnifiedDiffState::new();
        }
        Err(e) => {
            app.status_bar.set(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_uncommitted_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.uncommitted.diff_state = UnifiedDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.uncommitted.panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.uncommitted.panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.uncommitted.panel == Panel::Left {
                let len = app.uncommitted.file_map.len();
                if len > 0 {
                    let current = app.uncommitted.list_state.selected().unwrap_or(0);
                    let next = app
                        .uncommitted
                        .file_map
                        .iter()
                        .enumerate()
                        .skip(current + 1)
                        .find(|(_, x)| x.is_some())
                        .map(|(i, _)| i);
                    if let Some(next) = next {
                        app.uncommitted.list_state.select(Some(next));
                        load_uncommitted_diff(app, runner);
                    }
                }
            } else {
                app.uncommitted.diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.uncommitted.panel == Panel::Left {
                if let Some(current) = app.uncommitted.list_state.selected()
                    && current > 0
                {
                    let prev = app.uncommitted.file_map[..current]
                        .iter()
                        .rposition(|x| x.is_some());
                    if let Some(prev) = prev {
                        app.uncommitted.list_state.select(Some(prev));
                        load_uncommitted_diff(app, runner);
                    }
                }
            } else {
                app.uncommitted.diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('s') => match app.uncommitted.panel {
            Panel::Left => super::staging::stage_or_unstage(app, runner),
            Panel::Right => super::staging::stage_or_unstage_hunk(app, runner),
        },
        KeyCode::Char('c') => {
            app.return_to_uncommitted_detail = true;
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('J') => {
            app.uncommitted.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.uncommitted.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.uncommitted.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.uncommitted.diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.uncommitted.diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.uncommitted.diff_state.prev_hunk();
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        _ => {}
    }
}
```

Update tests — replace `app.panel` with `app.uncommitted.panel`, `app.current_diff` with `app.uncommitted.diff`, `app.status_message` with `app.status_bar.message`:

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
    fn test_esc_from_uncommitted_detail_preserves_diff() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.uncommitted.diff = Some(vec![]);

        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

        assert_eq!(app.mode, Mode::Normal);
        assert!(app.uncommitted.diff.is_some());
    }

    #[test]
    fn test_uncommitted_detail_panel_switch() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.uncommitted.panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.uncommitted.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.uncommitted.panel, Panel::Left);
    }

    #[test]
    fn test_uncommitted_detail_c_enters_commit_mode() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
        assert!(app.return_to_uncommitted_detail);
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

    #[test]
    fn test_enter_on_index_0_enters_uncommitted_detail() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.log_list_state.select(Some(0));

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
        app.log_list_state.select(Some(1));

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status parent1 abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
    }
}
```

- [ ] **Step 2: Update staging.rs field accesses**

Apply these replacements throughout `crates/gitat-ui/src/event/staging.rs`:

| Old | New |
|-----|-----|
| `app.uncommitted_list_state` | `app.uncommitted.list_state` |
| `app.uncommitted_file_map` | `app.uncommitted.file_map` |
| `app.current_diff` | `app.uncommitted.diff` |
| `app.diff_state` | `app.uncommitted.diff_state` |
| `app.set_status_message(` | `app.status_bar.set(` |

The full updated `selected_file_info`:

```rust
fn selected_file_info(app: &App) -> Option<(usize, bool)> {
    let visual_idx = app.uncommitted.list_state.selected()?;
    app.uncommitted.file_map.get(visual_idx).copied().flatten()
}
```

In `stage_or_unstage`, update:

```rust
            app.uncommitted.diff = None;
            app.uncommitted.diff_state = UnifiedDiffState::new();
```

In `stage_or_unstage_hunk`, update:

```rust
    let diff_files = match &app.uncommitted.diff {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };
    // ...
    let hunk_index = app.uncommitted.diff_state.current_hunk;
    // ...
                    app.uncommitted.diff = None;
                    app.uncommitted.diff_state = UnifiedDiffState::new();
                    // ...
                    if app.uncommitted.diff_state.current_hunk >= total_hunks {
                        app.uncommitted.diff_state.current_hunk = total_hunks.saturating_sub(1);
                    }
                    app.uncommitted.diff = Some(diff);
                // ...
                app.status_bar.set(format!("Failed to reload diff: {e}"));
                app.uncommitted.diff = None;
                app.uncommitted.diff_state = UnifiedDiffState::new();
```

In `load_diff_for_selected`, update:

```rust
            app.uncommitted.diff = Some(diff);
        // ...
            app.status_bar.set(format!("Failed to load diff: {e}"));
```

Update tests — replace `app.panel` with `app.uncommitted.panel`, `app.current_diff` with `app.uncommitted.diff`, `app.diff_state` with `app.uncommitted.diff_state`, `app.status_message` with `app.status_bar.message`:

```rust
    #[test]
    fn test_s_in_right_panel_calls_stage_hunk() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.uncommitted.panel = Panel::Right;
        // ...
        app.uncommitted.diff = Some(vec![gitat_core::diff::DiffFile { ... }]);
        app.uncommitted.diff_state.current_hunk = 0;
        // ...
        assert!(
            app.status_bar.message.is_none()
                || !app.status_bar.message.as_ref().unwrap().contains("failed")
        );
    }

    #[test]
    fn test_s_in_left_panel_still_stages_file() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.uncommitted.panel = Panel::Left;
        // ...
        assert!(app.status_bar.message.is_none());
    }

    #[test]
    fn test_s_in_uncommitted_detail_stages_file() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.uncommitted.panel = Panel::Left;
        // ...
        assert!(app.status_bar.message.is_none());
    }
```

### Task 4: Update event/mod.rs and event/normal.rs

**Files:**
- Modify: `crates/gitat-ui/src/event/mod.rs`
- Modify: `crates/gitat-ui/src/event/normal.rs`

- [ ] **Step 1: Update event/mod.rs field accesses**

Apply these replacements throughout `crates/gitat-ui/src/event/mod.rs`:

| Old | New |
|-----|-----|
| `app.filtered_log_indices` | `app.search.filtered_log_indices` |
| `app.pre_search_cursor` | `app.search.pre_search_cursor` |
| `app.conflict_state` | `app.conflict.editor_state` |
| `app.conflict_file` | `app.conflict.file` |
| `app.set_status_message(` | `app.status_bar.set(` |

In `load_log_preview`:

```rust
pub fn load_log_preview(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(ref indices) = app.search.filtered_log_indices {
        if let Some(sel) = app.log_list_state.selected()
            && let Some(&log_idx) = indices.get(sel)
        {
            commit_detail::load_commit_preview_at(app, runner, log_idx);
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

In `handle_search`:

```rust
fn handle_search(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            if let Some(cursor) = app.search.pre_search_cursor {
                app.log_list_state.select(Some(cursor));
            }
            app.search.filtered_log_indices = None;
            app.search.pre_search_cursor = None;
            app.mode = Mode::Normal;
            app.mode_stack.clear();
        }
        KeyCode::Enter => {
            let original_index = app.log_list_state.selected().and_then(|sel| {
                app.search
                    .filtered_log_indices
                    .as_ref()
                    .and_then(|indices| indices.get(sel).copied())
            });
            if let Some(idx) = original_index
                && commit_detail::prepare_commit_detail(app, runner, idx)
            {
                app.push_mode(Mode::CommitDetail);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app
                .search
                .filtered_log_indices
                .as_ref()
                .map_or(0, |v| v.len());
            if max > 0 {
                let current = app.log_list_state.selected().unwrap_or(0);
                let next = (current + 1).min(max - 1);
                app.log_list_state.select(Some(next));
            }
            load_log_preview(app, runner);
        }
        // ... rest stays the same
```

In `handle_conflict`:

```rust
fn handle_conflict(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    let (state, file) = match (&mut app.conflict.editor_state, &app.conflict.file) {
        (Some(state), Some(file)) => (state, file),
        _ => {
            app.mode = Mode::Normal;
            return;
        }
    };

    match key.code {
        // ... ours/theirs/next/prev stay the same ...
        KeyCode::Char('w') => {
            let content = state.result_content();
            let path = file.path.clone();
            if let Err(e) = gitat_core::conflict::resolve_file(runner, &path, &content) {
                app.status_bar.set(format!("Resolve failed: {e}"));
            } else {
                app.status_bar.set(format!("Resolved: {path}"));
                app.refresh(runner);
            }
            app.conflict.editor_state = None;
            app.conflict.file = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.conflict.editor_state = None;
            app.conflict.file = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Update event/mod.rs tests**

Apply these replacements in the test module:

| Old | New |
|-----|-----|
| `app.pre_search_cursor` | `app.search.pre_search_cursor` |
| `app.filtered_log_indices` | `app.search.filtered_log_indices` |
| `app.commit_detail_commit` | `app.commit_detail.commit` |
| `app.commit_detail_files` | `app.commit_detail.files` |

- [ ] **Step 3: Update event/normal.rs field accesses**

Apply these replacements throughout `crates/gitat-ui/src/event/normal.rs`:

| Old | New |
|-----|-----|
| `app.panel == Panel::Right` (in diff nav guards) | `app.uncommitted.panel == Panel::Right` |
| `app.diff_state.` | `app.uncommitted.diff_state.` |
| `app.panel = Panel::Right` | `app.uncommitted.panel = Panel::Right` |
| `app.panel = Panel::Left` | `app.uncommitted.panel = Panel::Left` |
| `app.pre_search_cursor` | `app.search.pre_search_cursor` |
| `app.set_status_message(` | `app.status_bar.set(` |

Remove the empty `'s'` handler (dead code):

```rust
        // DELETE these lines:
        // KeyCode::Char('s') => {
        //     // Staging is handled in UncommittedDetail mode, not in Normal mode
        // }
```

- [ ] **Step 4: Update event/normal.rs tests**

Replace `app.commit_detail_commit` with `app.commit_detail.commit`, `app.commit_detail_files` with `app.commit_detail.files`, `app.pre_search_cursor` with `app.search.pre_search_cursor`.

### Task 5: Update views/log.rs

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Update all field accesses**

Apply these replacements throughout `crates/gitat-ui/src/views/log.rs`:

| Old | New |
|-----|-----|
| `app.filtered_log_indices` | `app.search.filtered_log_indices` |
| `app.commit_detail_commit` | `app.commit_detail.commit` |
| `app.commit_detail_files` | `app.commit_detail.files` |
| `app.commit_detail_file_state` | `app.commit_detail.file_state` |
| `app.commit_detail_panel` | `app.commit_detail.panel` |
| `app.commit_detail_diff` | `app.commit_detail.diff` |
| `app.commit_detail_diff_state` | `app.commit_detail.diff_state` |
| `app.uncommitted_list_state` | `app.uncommitted.list_state` |
| `app.uncommitted_file_map` | `app.uncommitted.file_map` |
| `app.current_diff` | `app.uncommitted.diff` |
| `app.diff_state` | `app.uncommitted.diff_state` |
| `app.panel` (in render_uncommitted_file_list, render_uncommitted_diff) | `app.uncommitted.panel` |

These are mechanical find-and-replace operations across the entire file. Every function in log.rs is affected.

### Task 6: Update main.rs

**Files:**
- Modify: `crates/gitat/src/main.rs`

- [ ] **Step 1: Update field accesses in main.rs**

Apply these replacements in `crates/gitat/src/main.rs`:

| Old | New |
|-----|-----|
| `app.clear_expired_status_message()` | `app.status_bar.clear_if_expired()` |
| `app.status_message` | `app.status_bar.message` |
| `app.conflict_file` | `app.conflict.file` |
| `app.conflict_state` | `app.conflict.editor_state` |

In the `terminal.draw` closure:

```rust
            // Status bar
            let status_text = if let Some(ref msg) = app.status_bar.message {
                msg.clone()
            } else {
                "j/k: move  h/l: panel  Tab: switch  s: stage  c: commit  p: push  ?: help  q: quit"
                    .to_string()
            };
```

```rust
            // Conflict editor overlay
            if matches!(app.mode, Mode::Conflict { .. })
                && let (Some(file), Some(state)) = (&app.conflict.file, &mut app.conflict.editor_state)
            {
```

### Task 7: Verify Phase 1 — compile and run all tests

**Files:** None (verification only)

- [ ] **Step 1: Run cargo build**

Run: `cargo build 2>&1 | head -50`

Expected: Build succeeds with no errors. Fix any remaining field access issues if compilation fails.

- [ ] **Step 2: Run cargo test**

Run: `cargo test 2>&1`

Expected: All existing tests pass. If any test fails, the failure will be a compile error from a missed field rename — fix and re-run.

- [ ] **Step 3: Run cargo clippy**

Run: `cargo clippy 2>&1`

Expected: No new warnings. Fix any issues.

- [ ] **Step 4: Commit Phase 1**

```bash
git add -A
git commit -m "refactor: decompose App struct into domain sub-structures

Split 28-field App into CommitDetailState, UncommittedState, SearchState,
ConflictResolveState, and StatusBar sub-structures. All existing tests pass.

Task: gitat-ui refactoring Phase 1

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Phase 2: Status Filtering Helpers

### Task 8: Add status filtering helpers and update call sites

**Files:**
- Modify: `crates/gitat-ui/src/app.rs`
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Add helper methods to App**

Add these methods to `impl App` in `crates/gitat-ui/src/app.rs`:

```rust
    pub fn staged_entries(&self) -> Vec<(usize, &StatusEntry)> {
        self.status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.index_status != FileStatus::Unmodified
                    && e.index_status != FileStatus::Untracked
            })
            .collect()
    }

    pub fn modified_entries(&self) -> Vec<(usize, &StatusEntry)> {
        self.status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.worktree_status != FileStatus::Unmodified
                    && e.worktree_status != FileStatus::Untracked
            })
            .collect()
    }

    pub fn untracked_entries(&self) -> Vec<(usize, &StatusEntry)> {
        self.status
            .iter()
            .enumerate()
            .filter(|(_, e)| e.index_status == FileStatus::Untracked)
            .collect()
    }

    pub fn change_counts(&self) -> (usize, usize, usize) {
        let staged = self.status.iter().filter(|e| e.is_staged()).count();
        let unstaged = self
            .status
            .iter()
            .filter(|e| !e.is_staged() && e.worktree_status != FileStatus::Untracked)
            .count();
        let untracked = self
            .status
            .iter()
            .filter(|e| e.index_status == FileStatus::Untracked)
            .count();
        (staged, unstaged, untracked)
    }
```

- [ ] **Step 2: Update rebuild_uncommitted_file_map to use helpers**

Replace the body of `rebuild_uncommitted_file_map` in `crates/gitat-ui/src/app.rs`:

```rust
    pub fn rebuild_uncommitted_file_map(&mut self) {
        let mut map: Vec<Option<(usize, bool)>> = Vec::new();

        let staged: Vec<usize> = self.staged_entries().iter().map(|(i, _)| *i).collect();
        if !staged.is_empty() {
            map.push(None);
            for idx in staged {
                map.push(Some((idx, true)));
            }
        }

        let modified: Vec<usize> = self.modified_entries().iter().map(|(i, _)| *i).collect();
        if !modified.is_empty() {
            map.push(None);
            for idx in modified {
                map.push(Some((idx, false)));
            }
        }

        let untracked: Vec<usize> = self.untracked_entries().iter().map(|(i, _)| *i).collect();
        if !untracked.is_empty() {
            map.push(None);
            for idx in untracked {
                map.push(Some((idx, false)));
            }
        }

        self.uncommitted.file_map = map;
    }
```

- [ ] **Step 3: Update render_log_list_items in views/log.rs**

Replace the count calculations (lines ~86-97 in the original) with:

```rust
        let (staged_count, unstaged_count, untracked_count) = app.change_counts();
        let total_changes = staged_count + unstaged_count + untracked_count;
```

- [ ] **Step 4: Update render_uncommitted_preview in views/log.rs**

Replace the header count calculations and file list building. The header becomes:

```rust
fn render_uncommitted_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let (staged_count, unstaged_count, untracked_count) = app.change_counts();
    let total_changes = staged_count + unstaged_count + untracked_count;

    let header_text = uncommitted_header_text(staged_count, unstaged_count, untracked_count);
    let header = Paragraph::new(Line::from(Span::styled(
        header_text,
        Theme::border_focused(),
    )))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border()),
    );
    f.render_widget(header, chunks[0]);

    let items = build_status_file_items(app);

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
}
```

- [ ] **Step 5: Update render_uncommitted_detail header in views/log.rs**

Replace the header count calculations in `render_uncommitted_detail`:

```rust
    let (staged_count, unstaged_count, untracked_count) = app.change_counts();

    let header_text = uncommitted_header_text(staged_count, unstaged_count, untracked_count);
```

- [ ] **Step 6: Update render_uncommitted_file_list in views/log.rs**

Replace the file filtering with helper calls. The file list building becomes:

```rust
fn render_uncommitted_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.uncommitted.panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let items = build_status_file_items(app);

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.uncommitted.list_state);
}
```

- [ ] **Step 7: Run tests and commit Phase 2**

Run: `cargo test 2>&1`

Expected: All tests pass.

```bash
git add -A
git commit -m "refactor: extract status filtering helpers on App

Add staged_entries(), modified_entries(), untracked_entries(), and
change_counts() to eliminate 4+ duplicated filtering patterns.

Task: gitat-ui refactoring Phase 2

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Phase 3: Rendering Consolidation

### Task 9: Extract shared render functions

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Add shared helper functions**

Add these helper functions in `crates/gitat-ui/src/views/log.rs` (before the existing functions):

```rust
fn uncommitted_header_text(staged: usize, unstaged: usize, untracked: usize) -> String {
    if staged + unstaged + untracked > 0 {
        format!(
            "Uncommitted Changes — {} staged, {} unstaged",
            staged,
            unstaged + untracked
        )
    } else {
        "Uncommitted Changes".to_string()
    }
}

fn build_status_file_items(app: &App) -> Vec<ListItem> {
    let mut items: Vec<ListItem> = Vec::new();

    let staged = app.staged_entries();
    if !staged.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Staged",
            Theme::file_staged(),
        ))));
        for (_, entry) in &staged {
            let code = file_status_code(&entry.index_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_staged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    let modified = app.modified_entries();
    if !modified.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Modified",
            Theme::file_unstaged(),
        ))));
        for (_, entry) in &modified {
            let code = file_status_code(&entry.worktree_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_unstaged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    let untracked = app.untracked_entries();
    if !untracked.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Untracked",
            Theme::file_untracked(),
        ))));
        for (_, entry) in &untracked {
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

    items
}

fn render_diff_panel(
    f: &mut Frame,
    diff: Option<&Vec<DiffFile>>,
    diff_state: &mut UnifiedDiffState,
    is_focused: bool,
    area: Rect,
) {
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(diff_files) = diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let widget = UnifiedDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, diff_state);
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

- [ ] **Step 2: Replace render_commit_diff with render_diff_panel call**

Replace the `render_commit_diff` function body and its call site. In `render_commit_detail`, replace:

```rust
    render_file_list(f, app, panels[0]);
    render_diff_panel(
        f,
        app.commit_detail.diff.as_ref(),
        &mut app.commit_detail.diff_state,
        app.commit_detail.panel == Panel::Right,
        panels[1],
    );
```

Delete the old `render_commit_diff` function entirely.

- [ ] **Step 3: Replace render_uncommitted_diff with render_diff_panel call**

In `render_uncommitted_detail`, replace:

```rust
    render_uncommitted_file_list(f, app, panels[0]);
    render_diff_panel(
        f,
        app.uncommitted.diff.as_ref(),
        &mut app.uncommitted.diff_state,
        app.uncommitted.panel == Panel::Right,
        panels[1],
    );
```

Delete the old `render_uncommitted_diff` function entirely.

- [ ] **Step 4: Add DiffFile import if needed**

Ensure `use gitat_core::diff::DiffFile;` is in the imports at the top of `views/log.rs`. Also add `use crate::widgets::unified_diff::UnifiedDiffState;` if not already present.

- [ ] **Step 5: Run tests and commit Phase 3**

Run: `cargo test 2>&1`

Expected: All tests pass.

```bash
git add -A
git commit -m "refactor: consolidate rendering — shared diff panel, file list, header

Extract render_diff_panel(), build_status_file_items(), and
uncommitted_header_text() to eliminate duplicated rendering code.

Task: gitat-ui refactoring Phase 3

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Phase 4: Polish

### Task 10: Constants, dead code removal, final cleanup

**Files:**
- Modify: `crates/gitat-ui/src/views/log.rs`

- [ ] **Step 1: Add layout constants to views/log.rs**

Add at the top of `crates/gitat-ui/src/views/log.rs` (after imports):

```rust
const LOG_LIST_PERCENT: u16 = 60;
const PREVIEW_PERCENT: u16 = 40;
const FILE_LIST_PERCENT: u16 = 30;
const DIFF_PANEL_PERCENT: u16 = 70;
```

- [ ] **Step 2: Replace magic numbers with constants**

In `render_log_list`:

```rust
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(LOG_LIST_PERCENT),
            Constraint::Percentage(PREVIEW_PERCENT),
        ])
        .split(main_area);
```

In `render_commit_detail`:

```rust
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(FILE_LIST_PERCENT),
            Constraint::Percentage(DIFF_PANEL_PERCENT),
        ])
        .split(chunks[1]);
```

In `render_uncommitted_detail`:

```rust
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(FILE_LIST_PERCENT),
            Constraint::Percentage(DIFF_PANEL_PERCENT),
        ])
        .split(chunks[1]);
```

- [ ] **Step 3: Run tests and commit Phase 4**

Run: `cargo test 2>&1`

Expected: All tests pass.

Run: `cargo clippy 2>&1`

Expected: No warnings.

```bash
git add -A
git commit -m "refactor: extract layout constants, remove dead code

Replace magic numbers with named constants for layout percentages.
Remove dead 's' handler in normal mode.

Task: gitat-ui refactoring Phase 4

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

---

## Final Verification

### Task 11: Full test suite and manual smoke test

- [ ] **Step 1: Run full test suite**

Run: `cargo test 2>&1`

Expected: All tests pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy 2>&1`

Expected: No warnings.

- [ ] **Step 3: Run fmt check**

Run: `cargo fmt -- --check 2>&1`

Expected: No formatting issues.

- [ ] **Step 4: Build release**

Run: `cargo build --release 2>&1`

Expected: Build succeeds.
