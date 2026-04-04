# gitat Refactoring Design

## Overview

Comprehensive refactoring of the gitat-ui crate to address structural issues: a god struct (`App` with 28 fields), duplicated code patterns, and tangled responsibilities. The approach is top-down — restructure first, then consolidate.

## Motivation

- `App` struct has grown to 28 public fields mixing UI state, data, and detail view state
- Status filtering logic (staged/modified/untracked) is duplicated in 4+ locations
- Diff panel rendering is nearly identical in `render_commit_diff` and `render_uncommitted_diff`
- File list rendering is duplicated between `render_uncommitted_preview` and `render_uncommitted_file_list`
- Magic numbers scattered across layout code
- Dead code in event handlers

## Phase 1: App Struct Decomposition

Split `App` into domain-specific sub-structures.

### New Sub-Structures

```rust
pub struct CommitDetailState {
    pub commit: Option<CommitInfo>,
    pub files: Vec<CommitFileEntry>,
    pub file_state: ListState,
    pub panel: Panel,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
}

pub struct UncommittedState {
    pub list_state: ListState,
    pub file_map: Vec<Option<(usize, bool)>>,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
    pub panel: Panel,
}

pub struct SearchState {
    pub pre_search_cursor: Option<usize>,
    pub filtered_log_indices: Option<Vec<usize>>,
}

pub struct ConflictResolveState {
    pub editor_state: Option<ConflictEditorState>,
    pub file: Option<ConflictFile>,
}

pub struct StatusBar {
    pub message: Option<String>,
    pub set_at: Option<Instant>,
}
```

### Refactored App

```rust
pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub mode_stack: Vec<Mode>,
    pub should_quit: bool,
    pub return_to_uncommitted_detail: bool,

    // Data
    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log_entries: Vec<CommitInfo>,

    // List states
    pub log_list_state: ListState,
    pub branches_list_state: ListState,

    // Sub-states
    pub uncommitted: UncommittedState,
    pub commit_detail: CommitDetailState,
    pub search: SearchState,
    pub conflict: ConflictResolveState,
    pub status_bar: StatusBar,
}
```

### Field Migration Map

| Old field | New location |
|-----------|-------------|
| `panel` | `uncommitted.panel` |
| `uncommitted_list_state` | `uncommitted.list_state` |
| `uncommitted_file_map` | `uncommitted.file_map` |
| `current_diff` | `uncommitted.diff` |
| `diff_state` | `uncommitted.diff_state` |
| `commit_detail_commit` | `commit_detail.commit` |
| `commit_detail_files` | `commit_detail.files` |
| `commit_detail_file_state` | `commit_detail.file_state` |
| `commit_detail_panel` | `commit_detail.panel` |
| `commit_detail_diff` | `commit_detail.diff` |
| `commit_detail_diff_state` | `commit_detail.diff_state` |
| `pre_search_cursor` | `search.pre_search_cursor` |
| `filtered_log_indices` | `search.filtered_log_indices` |
| `conflict_state` | `conflict.editor_state` |
| `conflict_file` | `conflict.file` |
| `status_message` | `status_bar.message` |
| `status_message_set_at` | `status_bar.set_at` |

### Impact

All files in gitat-ui that access App fields will need updating: event handlers, views, and the main binary.

## Phase 2: Status Filtering Helpers

Add methods to `App` that centralize status entry filtering:

```rust
impl App {
    pub fn staged_entries(&self) -> Vec<(usize, &StatusEntry)> { ... }
    pub fn modified_entries(&self) -> Vec<(usize, &StatusEntry)> { ... }
    pub fn untracked_entries(&self) -> Vec<(usize, &StatusEntry)> { ... }
    pub fn change_counts(&self) -> (usize, usize, usize) { ... }
}
```

### Call sites to update

- `app.rs`: `rebuild_uncommitted_file_map` (move to `UncommittedState` method)
- `views/log.rs`: `render_log_list_items` (count calculations)
- `views/log.rs`: `render_uncommitted_preview` (file list building)
- `views/log.rs`: `render_uncommitted_file_list` (file list building)
- `views/log.rs`: `render_uncommitted_detail` (header counts)

## Phase 3: Rendering Consolidation

### 3a. Shared Diff Panel Renderer

Extract `render_commit_diff` and `render_uncommitted_diff` into a single function:

```rust
fn render_diff_panel(
    f: &mut Frame,
    diff: Option<&Vec<DiffFile>>,
    diff_state: &mut UnifiedDiffState,
    is_focused: bool,
    area: Rect,
)
```

### 3b. Shared File Status List Builder

Extract the repeated staged/modified/untracked section rendering into:

```rust
fn build_status_file_items(
    staged: &[(usize, &StatusEntry)],
    modified: &[(usize, &StatusEntry)],
    untracked: &[(usize, &StatusEntry)],
) -> Vec<ListItem>
```

Used by both `render_uncommitted_preview` and `render_uncommitted_file_list`.

### 3c. Shared Uncommitted Header

```rust
fn uncommitted_header_text(staged: usize, unstaged: usize, untracked: usize) -> String
```

Used by `render_uncommitted_preview` and `render_uncommitted_detail`.

## Phase 4: Polish

### 4a. Constants

```rust
// views/log.rs
const LOG_LIST_PERCENT: u16 = 60;
const PREVIEW_PERCENT: u16 = 40;
const FILE_LIST_PERCENT: u16 = 30;
const DIFF_PANEL_PERCENT: u16 = 70;

// app.rs
const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(3);
```

### 4b. Dead Code Removal

- Remove empty `'s'` handler in `event/normal.rs`

### 4c. StatusBar Methods

Move `set_status_message` and `clear_expired_status_message` logic to `StatusBar`:

```rust
impl StatusBar {
    pub fn set(&mut self, msg: impl Into<String>) { ... }
    pub fn clear_if_expired(&mut self) { ... }
}
```

All callers migrate to `app.status_bar.set(...)` directly. No wrapper methods on `App`.

## Testing Strategy

- All existing tests must pass after each phase
- Run `cargo test` after each phase to verify
- Run `cargo clippy` to catch dead code or unused imports
- Snapshot tests in `insta` should not change (rendering output is preserved)

## Out of Scope

- Splitting `unified_diff.rs` (897 lines) into sub-modules — can be a follow-up
- Trait-based mode handler pattern for `event/mod.rs` — can be a follow-up
- Test reorganization into separate directories — can be a follow-up
