# Log Commit Preview Panel Design

## Summary

Add an inline commit detail preview to the bottom 40% of the Log tab screen. The preview is read-only and shows the same layout as the full-screen CommitDetail/UncommittedDetail views. Pressing Enter expands to the existing full-screen mode.

## Requirements

- Log tab Normal mode splits vertically: top 60% log list, bottom 40% preview
- Preview shows file list (30%) + diff (70%) in left-right split, matching full-screen layout
- Preview is read-only: no panel focus, no scroll, no key bindings
- Commit selection (index > 0): shows commit metadata header, file list, and first file's diff
- Uncommitted selection (index == 0): shows staged/modified/untracked file list and first file's diff
- Enter key transitions to full-screen CommitDetail or UncommittedDetail (existing behavior preserved)
- Preview data loads on cursor movement (j/k) and on initial tab display

## Layout

```
+------------------------------+  Tab bar (1 line)
+------------------------------+
|                              |
|   Log list (60%)             |  Existing commit list
|                              |
+------------------------------+
| File list (30%) | Diff (70%) |  Preview panel (40%)
|                 |            |  Same layout as CommitDetail
+------------------------------+
  Status bar (1 line)
```

## Approach

Modify `render_log_list()` in Normal mode to split the area vertically. No new Mode variants are added. Existing `app.commit_detail_*` and `app.status` / `app.current_diff` fields are reused for preview data.

## Data Flow and State Management

### Data Loading

- On cursor move (j/k) in Log list, load preview data for the selected item:
  - **Commit (index > 0)**: Call `commit_detail::get_commit_files()` to populate `app.commit_detail_files`. Load diff for the first file into `app.commit_detail_diff`.
  - **Uncommitted (index == 0)**: Use existing `app.status` data. Load diff for the first file into `app.current_diff`.
- On initial Log tab display, load preview data for the default selection (index 0).

### State Reuse

- No new App fields are added.
- Preview and full-screen modes share the same data fields (`commit_detail_commit`, `commit_detail_files`, `commit_detail_diff`, `status`, `current_diff`).
- Enter transitions to full-screen without re-fetching data.

### Performance

- Each cursor move triggers git commands (same cost as current CommitDetail entry).
- Acceptable for initial implementation; caching can be added later if needed.

## Event Handling Changes

### Normal Mode (event/normal.rs)

- **j/k**: After list move, trigger preview data load for newly selected item.
- **Enter**: Unchanged. Transitions to `Mode::CommitDetail` or `Mode::UncommittedDetail`.
- **Initial load**: When entering Log tab, load preview data for current selection.

### No New Key Bindings

Preview is read-only. No panel switching or scrolling in preview mode.

### CommitDetail/UncommittedDetail Modes

No changes. Esc returns to Normal mode with preview data intact.

## Rendering Changes

### render_log_list() Modification

Split `area` using `Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)])`:
- Top: Existing log list rendering (unchanged)
- Bottom: Dispatch to preview helper based on selection

### New Helper Functions (in views/log.rs)

- `render_commit_preview(f, app, area)`:
  - Top rows: Commit metadata (hash, author, date, message)
  - Left 30%: File list from `app.commit_detail_files` (no selection highlight)
  - Right 70%: Diff from `app.commit_detail_diff` (scroll position fixed at top)

- `render_uncommitted_preview(f, app, area)`:
  - Top row: Header with change counts
  - Left 30%: File list from `app.status` grouped by section (no selection highlight)
  - Right 70%: Diff from `app.current_diff` (scroll position fixed at top)

### Preview vs Full-Screen Differences

| Aspect          | Preview              | Full-Screen          |
|-----------------|----------------------|----------------------|
| File list       | No selection highlight | Selection highlight  |
| Diff scroll     | Fixed at top         | Scrollable           |
| Key bindings    | None                 | h/l/j/k panel ops   |
| Area            | Bottom 40%           | Entire content area  |

## Files to Modify

| File | Change |
|------|--------|
| `crates/gitat-ui/src/views/log.rs` | Split layout in `render_log_list()`, add `render_commit_preview()` and `render_uncommitted_preview()` |
| `crates/gitat-ui/src/event/normal.rs` | Add preview data loading on j/k cursor move and initial tab entry |
| `crates/gitat-ui/src/event/commit_detail.rs` | Possibly extract data loading into reusable function |
| `crates/gitat-ui/src/event/uncommitted_detail.rs` | Possibly extract data loading into reusable function |
