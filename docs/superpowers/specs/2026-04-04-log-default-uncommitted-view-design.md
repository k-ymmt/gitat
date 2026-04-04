# Design: Log as Default Screen with Uncommitted Changes View

**Date:** 2026-04-04
**Status:** Approved

## Summary

Replace the Status tab with a unified Log-centric workflow. The Log tab becomes the default screen, with an always-visible "Uncommitted Changes" item at the top of the log list. Entering this item opens a 2-column detail view that provides full staging, unstaging, and commit capabilities — effectively absorbing the Status tab's functionality.

## Requirements

1. **Default screen is Log** — app launches to the Log tab
2. **Tab order:** Log | Branches | Stash (Status tab removed)
3. **Uncommitted Changes item** — always displayed as the first entry in the Log list, regardless of working tree state
4. **Display format:**
   - With changes: `Uncommitted Changes — N staged, M unstaged`
   - Clean working tree: `Uncommitted Changes` (no counts)
5. **UncommittedDetail view** — opened by pressing Enter on the uncommitted item; provides a 2-column layout identical in structure to CommitDetail but with staging/commit capabilities
6. **Status tab is fully removed** — all its functionality moves into the UncommittedDetail view

## Architecture

### Approach: New `Mode::UncommittedDetail`

Add a new mode parallel to `Mode::CommitDetail`. This maintains the existing mode-based architecture and keeps each mode's responsibilities clear.

### Mode Enum Change

```rust
enum Mode {
    Normal,
    Commit { message: String },
    Help,
    Search { query: String },
    Conflict { file: String },
    CommitDetail,
    UncommittedDetail,  // NEW
}
```

### Tab Enum Change

```rust
enum Tab {
    Log,       // default, was previously third
    Branches,
    Stash,
}
```

`Tab::default()` returns `Log`.

## Log List Rendering

### Uncommitted Item at Top

- The uncommitted item is rendered as the first entry before any `log_entries`
- It is visually distinct: uses Cyan color for the label (instead of yellow hash)
- Staged/unstaged counts are computed from `app.status`
- When selected (cursor at index 0), the selection highlight applies normally

### Index Management

- List index 0 = uncommitted item
- List index 1..N = `log_entries[0..N-1]`
- All code accessing `log_entries` from list index must subtract 1
- Total list length = `1 + log_entries.len()`

## UncommittedDetail View

### Layout

```
┌─────────────────────────────────────────────────┐
│ [ Log ] [ Branches ] [ Stash ]                  │
├─────────────────────────────────────────────────┤
│ Uncommitted Changes — 2 staged, 1 unstaged      │
├──────────────┬──────────────────────────────────┤
│ Staged       │                                  │
│  M src/app.rs│  1  use std::collections::Ha...  │
│  A new.rs    │  2 -use std::path::Path;         │
│ Modified     │  2 +use std::path::PathBuf;      │
│  M log.rs    │  3  use anyhow::Result;          │
│ Untracked    │  ...                             │
│  ? TODO.txt  │                                  │
├──────────────┴──────────────────────────────────┤
│ s: stage/unstage  c: commit  Esc: back          │
└─────────────────────────────────────────────────┘
```

- **Header:** 1 line showing "Uncommitted Changes — N staged, M unstaged"
- **Left panel (30%):** File list with 3 sections (Staged / Modified / Untracked)
- **Right panel (70%):** Side-by-side diff viewer (`SideBySideDiff` widget)
- Left panel border is Cyan when focused, right panel border is Cyan when focused

### File List Sections

Reuses the same 3-section rendering logic currently in `views/status.rs`:
- **Staged** (green header): files with `index_status != Unmodified`
- **Modified** (red header): files with `worktree_status != Unmodified` and not untracked
- **Untracked** (gray header): files with `Untracked` status

### Diff Display

- Selecting a file in the left panel loads its diff into `app.current_diff`
- Staged files use `git diff --cached`, unstaged files use `git diff`
- `SideBySideDiff` widget renders the diff with word-level highlighting
- Empty state (no files or clean tree): display placeholder text

## Key Bindings in UncommittedDetail Mode

| Key | Panel | Action |
|-----|-------|--------|
| `h` | any | Focus left panel |
| `l` | any | Focus right panel |
| `j`/`Down` | left | Select next file (auto-loads diff) |
| `k`/`Up` | left | Select previous file (auto-loads diff) |
| `s` | left | Stage/unstage selected file |
| `s` | right | Stage/unstage current hunk |
| `n` | right | Jump to next hunk |
| `N` | right | Jump to previous hunk |
| `J` | right | Scroll down |
| `K` | right | Scroll up |
| `H` | right | Scroll left |
| `L` | right | Scroll right |
| `c` | any | Enter commit mode (modal popup) |
| `Esc` | any | Return to Log list (`Mode::Normal`) |
| `?` | any | Toggle help |

## Event Handling

### New File: `event/uncommitted_detail.rs`

- `handle_uncommitted_detail(app, key, runner)` — main dispatcher for UncommittedDetail mode
- `enter_uncommitted_detail(app, runner)` — transition into the mode (load first file's diff, reset diff state)
- Reuses `event/staging.rs` functions: `stage_or_unstage()`, `stage_or_unstage_hunk()`

### Event Dispatcher Update (`event/mod.rs`)

Add `Mode::UncommittedDetail` to the match in the event dispatcher, routing to `uncommitted_detail::handle_uncommitted_detail()`.

### Staging Post-Actions

After any stage/unstage operation:
1. `app.refresh(runner)` — re-fetch status, log, branches
2. Reload diff for the currently selected file
3. Clamp file list cursor if needed (file may have moved between sections)
4. Header counts update automatically (derived from `app.status`)

### Commit Post-Actions

After successful commit:
1. `app.refresh(runner)` — re-fetch all data
2. Remain in `Mode::UncommittedDetail` (user can continue working)
3. If all changes are committed, show clean/empty state

## Log View Rendering Update (`views/log.rs`)

### Normal Mode (Log List)

- Render uncommitted item at index 0 before iterating `log_entries`
- Handle `Mode::UncommittedDetail` as an additional rendering branch (alongside `Mode::CommitDetail`)

### Enter Key Dispatch

- Index 0 + Enter → `enter_uncommitted_detail()`
- Index >= 1 + Enter → `enter_commit_detail()` (existing behavior)

## Removals

| Item | Action |
|------|--------|
| `Tab::Status` | Remove enum variant |
| `views/status.rs` | Delete file (rendering logic migrated to `views/log.rs` UncommittedDetail section) |
| `status_list_state` in `App` | Rename to `uncommitted_list_state` (same type, reused for file list in UncommittedDetail) |
| Status-specific code in `event/normal.rs` | Remove (replaced by UncommittedDetail handling) |
| `main.rs` Status tab rendering | Remove |

### Preserved

| Item | Reason |
|------|--------|
| `event/staging.rs` | Shared logic, used by UncommittedDetail |
| `app.status: Vec<StatusEntry>` | Still needed for uncommitted item data |
| `app.current_diff` | Reused for diff display in UncommittedDetail |
| `app.diff_state` | Reused for diff scrolling/hunk navigation |

## Empty State

When the working tree is clean (no staged, modified, or untracked files):
- Log list shows `Uncommitted Changes` without counts
- Entering UncommittedDetail shows empty left panel and placeholder text in right panel (e.g., "No uncommitted changes")

## Testing Strategy

- **Unit tests:** Verify index offset logic (index 0 = uncommitted, index N = log_entries[N-1])
- **Unit tests:** Verify staged/unstaged count computation from `Vec<StatusEntry>`
- **Unit tests:** Verify mode transitions (Enter on index 0 vs index >= 1)
- **Integration tests:** Stage/unstage operations update status correctly
- **Snapshot tests:** Log list rendering with and without uncommitted changes
- **Snapshot tests:** UncommittedDetail layout with various file states
