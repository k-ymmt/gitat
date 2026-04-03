# Commit Detail View Design

## Overview

Add a commit detail view accessible from the Log tab. When the user selects a commit and presses Enter, the view transitions to a two-panel layout showing commit metadata, changed files, and side-by-side diff for the selected file. Esc returns to the Log list.

## Data Layer (`gitat-core`)

### New Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CommitFileEntry {
    pub path: String,
    pub status: FileChangeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}
```

### New Functions

**Get changed files for a commit:**
- Command: `git diff-tree --no-commit-id -r --name-status <hash>`
- Signature: `pub fn get_commit_files(runner: &dyn CommandRunner, hash: &str) -> Result<Vec<CommitFileEntry>, GitError>`
- Parses each line as `<status>\t<path>`

**Get diff for a specific file in a commit:**
- Command: `git diff <parent_hash>..<hash> -- <file_path>`
- For root commits (no parent): `git diff --root <hash> -- <file_path>` or `git show <hash> -- <file_path>`
- Signature: `pub fn get_commit_file_diff(runner: &dyn CommandRunner, hash: &str, parent_hash: Option<&str>, file_path: &str) -> Result<Vec<DiffFile>, GitError>`
- Reuses existing `diff::parse_diff` for parsing

## State Management (`gitat-ui`)

### Mode Extension

Add `CommitDetail` variant to the `Mode` enum:

```rust
pub enum Mode {
    Normal,
    Commit { message: String },
    Search { query: String },
    Help,
    Conflict { file: String },
    CommitDetail,
}
```

### App State Fields

Add the following fields to `App`:

```rust
pub commit_detail_commit: Option<CommitInfo>,
pub commit_detail_files: Vec<CommitFileEntry>,
pub commit_detail_file_state: ListState,
pub commit_detail_panel: Panel,
pub commit_detail_diff: Option<Vec<DiffFile>>,
pub commit_detail_diff_state: SideBySideDiffState,
```

### Transition Flow

1. **Enter (Log tab, Normal mode):** Get selected commit from `log_entries` via `log_list_state`. Fetch changed files via `get_commit_files`. Set `commit_detail_commit`, `commit_detail_files`, `commit_detail_file_state` (select first file), `commit_detail_panel = Left`. Auto-load diff for first file. Switch to `Mode::CommitDetail`.
2. **Esc (CommitDetail mode):** Switch to `Mode::Normal`. Clear commit detail state fields.

## UI Layout

```
┌─ Commit Detail ──────────────────────────────────────┐
│ [Metadata - 3 lines]                                  │
│  Hash: abc1234  Author: k-ymmt  Date: 2026-04-01     │
│  Message: fix: resolve parsing bug                    │
├──────────────────────┬───────────────────────────────┤
│ [Left: File list]     │ [Right: Side-by-side diff]    │
│  M  src/main.rs       │  old line  │  new line        │
│  A  src/util.rs       │  ...       │  ...             │
│  D  src/old.rs        │            │                  │
└──────────────────────┴───────────────────────────────┘
```

### Rendering Details

- **Metadata area (top, fixed 3 lines):** Hash (yellow), Author, Date on first line. Commit message on second line. Blank separator on third line.
- **Left panel (50% width):** `List` widget showing changed files. File status colored: Added=green, Modified=yellow, Deleted=red. Highlight selected file.
- **Right panel (50% width):** Reuse existing `SideBySideDiff` widget. Display diff for the currently selected file.
- **Rendering location:** Inside `views/log.rs`. When `app.mode` is `CommitDetail`, render detail view instead of commit list.
- **Auto-load diff:** When file selection changes (j/k in left panel), automatically fetch and display the diff for the newly selected file.

## Key Bindings

New `handle_commit_detail` function in `event.rs`:

| Key | Action |
|-----|--------|
| `Esc` | Return to `Mode::Normal` (Log list) |
| `h` | Focus left panel (file list) |
| `l` | Focus right panel (diff) |
| `j` / `Down` | Left: move file selection down + auto-load diff. Right: scroll diff down |
| `k` / `Up` | Left: move file selection up + auto-load diff. Right: scroll diff up |
| `H` / `J` / `K` / `L` | Scroll diff in right panel (same as Status view) |
| `n` / `N` | Next/previous diff hunk |
| `q` | Quit application |

### Event Dispatch

Add `Mode::CommitDetail` arm to `handle_key` match in `event.rs`:

```rust
Mode::CommitDetail => handle_commit_detail(app, key, runner),
```

## Files to Modify

- `crates/gitat-core/src/lib.rs` — export new module/types
- `crates/gitat-core/src/log.rs` (or new file) — `CommitFileEntry`, `FileChangeStatus`, `get_commit_files`, `get_commit_file_diff`
- `crates/gitat-ui/src/app.rs` — `Mode::CommitDetail`, new App fields, initialization
- `crates/gitat-ui/src/event.rs` — `handle_commit_detail`, modify Enter in Log tab, add Mode dispatch
- `crates/gitat-ui/src/views/log.rs` — render commit detail view
- `crates/gitat-ui/src/theme.rs` — colors for file change status (if not already covered)

## Testing

- Unit tests for `get_commit_files` parsing
- Unit tests for `get_commit_file_diff` (mock runner)
- Snapshot tests for commit detail view rendering
