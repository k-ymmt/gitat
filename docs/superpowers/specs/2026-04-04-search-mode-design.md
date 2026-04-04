# Search Mode (`/`) — Incremental Filtering

## Summary

Implement incremental filtering for the log list via `/` search mode. Users type a query and the log list filters in real-time to show only matching commits. Enter confirms the selection; Esc cancels.

## Search Targets

Case-insensitive substring match against:
- `CommitInfo.message`
- `CommitInfo.short_hash`
- `CommitInfo.author`

## Data Model

Add to `App`:

```rust
/// Original cursor position before entering search mode, for Esc restoration.
pub pre_search_cursor: Option<usize>,

/// Indices into `log_entries` that match the current query.
/// `None` = no filter (normal display). `Some(vec)` = filtered view.
pub filtered_log_indices: Option<Vec<usize>>,
```

`log_entries` is never modified by search. `filtered_log_indices` stores references back into it.

## Filter Logic

On every keystroke that modifies the query (`Char`, `Backspace`):

1. Iterate `log_entries` and collect indices where any of the three fields contains the query (case-insensitive).
2. Store result in `filtered_log_indices`.
3. Reset `log_list_state` selection to 0.
4. If query is empty, set `filtered_log_indices` to `None` (show all).

## Key Handling in Search Mode

| Key       | Action |
|-----------|--------|
| `Char(c)` | Append to query, re-filter |
| `Backspace`| Pop from query, re-filter |
| `Enter`   | Confirm: resolve selected filtered index to original index, set `log_list_state` selection, clear filter, return to `Mode::Normal` |
| `Esc`     | Cancel: restore `pre_search_cursor` to `log_list_state`, clear filter, return to `Mode::Normal` |

## Search UI

Render an inline search bar at the bottom of the log list area when `Mode::Search`:

```
/ query_
```

The `_` cursor indicator follows the current query text. Rendered as a single-line `Paragraph` in the footer area of the log tab layout.

## Rendering Changes

In `render_log_list_items` (`views/log.rs`):

- When `filtered_log_indices` is `Some(indices)`:
  - Build list items from `indices.iter().map(|&i| &app.log_entries[i])` instead of iterating all entries.
  - The "uncommitted changes" row (index 0 in normal view) is excluded from filtered results — search only applies to commits.
  - Graph columns are omitted during filtered view to avoid visual artifacts from non-contiguous commits.
- When `filtered_log_indices` is `None`:
  - Normal rendering (unchanged).

## Enter Search Mode

In `normal.rs`, the existing `/` handler:

```rust
KeyCode::Char('/') => {
    app.pre_search_cursor = app.log_list_state.selected();
    app.mode = Mode::Search { query: String::new() };
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/gitat-ui/src/app.rs` | Add `pre_search_cursor` and `filtered_log_indices` fields |
| `crates/gitat-ui/src/event/mod.rs` | Expand `handle_search` with filter + Enter/Esc logic |
| `crates/gitat-ui/src/event/normal.rs` | Save cursor position on `/` |
| `crates/gitat-ui/src/views/log.rs` | Conditional rendering with filtered indices, search bar |
| `crates/gitat/src/main.rs` | Render search bar in draw closure (if not handled in log view) |

## Testing

- Unit test: filter logic produces correct indices for a given query and log entries.
- Snapshot test: rendered log list with active search filter.
- Snapshot test: search bar rendering.
