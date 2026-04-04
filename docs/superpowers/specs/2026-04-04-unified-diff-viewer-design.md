# Unified (1-Column) Diff Viewer Design

## Summary

Replace the current 2-column side-by-side diff widget with a 1-column unified diff viewer in the GitHub PR style. Removed and added lines are displayed vertically with full-width colored backgrounds and word-level highlighting on paired changes.

## Motivation

The side-by-side layout halves the available content width per side, making long lines harder to read. A unified 1-column layout uses the full terminal width and provides a familiar GitHub PR-style reading experience.

## Approach

Rename and refactor the existing `SideBySideDiff` widget in-place rather than creating a new widget from scratch. This preserves the existing state management, line pairing logic, word-level diff computation, and test infrastructure.

## Widget Renaming

| Current | New |
|---------|-----|
| `side_by_side_diff.rs` | `unified_diff.rs` |
| `SideBySideDiff` | `UnifiedDiff` |
| `SideBySideDiffState` | `UnifiedDiffState` |

## State Structure (unchanged)

`UnifiedDiffState` retains all existing fields and methods:

- `scroll_y: u16` — Vertical scroll position
- `scroll_x: u16` — Horizontal scroll position
- `current_hunk: usize` — Currently selected hunk
- `hunk_offsets: Vec<u16>` — Offset of each hunk in rendered output
- Methods: `scroll_down/up`, `scroll_right/left`, `next_hunk/prev_hunk`

## Preserved Logic

- `DiffRow` struct and `pair_lines()` function — unchanged
- `render_content_with_word_diff()` — word-level diff using `similar::TextDiff::from_words()`
- `write_segments()` — character rendering with CJK width handling

## Rendering Layout

### Current (2-column)

```
[OldLN(4)] [Left Content]  │  [NewLN(4)] [Right Content]
```

Content width per side: `(terminal_width / 2) - 5`

### New (1-column)

```
[OldLN(4)] [NewLN(4)] │ Content
```

Content width: `terminal_width - 9`

### Line Type Rendering

| Line Type   | OldLN   | NewLN   | Background        | Text Style            |
|-------------|---------|---------|-------------------|-----------------------|
| Context     | display | display | none              | `diff_context()` gray |
| Removed     | display | blank   | `diff_removed()` bg | `diff_removed()` fg |
| Added       | blank   | display | `diff_added()` bg  | `diff_added()` fg   |
| Hunk header | —       | —       | `diff_hunk_header()` | cyan               |

### DiffRow Expansion

In the current 2-column layout, one `DiffRow` maps to one screen row (left + right).

In the new 1-column layout, one `DiffRow` expands to up to 2 screen rows:
1. The removed line (if present) — rendered with red background
2. The added line (if present) — rendered with green background

Context rows expand to exactly 1 screen row.

Word-level highlighting is applied to paired removed/added lines within the same `DiffRow`, using the existing `similar::TextDiff::from_words()` logic.

## Callsite Changes

All changes are type renames only — no behavioral changes:

### `app.rs`

- `diff_state: SideBySideDiffState` → `diff_state: UnifiedDiffState`
- `commit_detail_diff_state: SideBySideDiffState` → `commit_detail_diff_state: UnifiedDiffState`

### `views/log.rs`

- Widget type: `SideBySideDiff` → `UnifiedDiff`
- State type references updated

### `event/commit_detail.rs`

- State type references updated

### `event/uncommitted_detail.rs`

- State type references updated

### `widgets/mod.rs`

- Module and re-export renamed

## Key Bindings (unchanged)

- `j/k` — Scroll down/up
- `H/J/K/L` — Scroll left/right
- `n/N` — Jump to next/previous hunk
- `s` — Stage/unstage hunk (Status tab, right panel only)

## Testing

### Preserved Tests

- `pair_lines` unit tests — logic unchanged

### Updated Snapshot Tests

All rendering snapshots need regeneration via `cargo insta review`:

- `snapshot_render_context_and_changes`
- `snapshot_render_cjk_characters`
- `snapshot_render_with_horizontal_scroll`
- `snapshot_render_narrow_terminal`
- `snapshot_render_multiple_hunks`

Visual verification of 1-column layout correctness during `insta review`.

## Hunk Offset Calculation

`hunk_offsets` must account for the new row expansion. In 2-column mode, each `DiffRow` was 1 screen row. In 1-column mode, paired rows (removed + added) produce 2 screen rows. The offset calculation in the render method must be updated accordingly.

## Theme (unchanged)

All existing theme functions are reused as-is:

- `diff_added()` / `diff_removed()` — line-level colors
- `diff_word_added()` / `diff_word_removed()` — word-level highlight
- `diff_context()` — context line color
- `diff_line_number()` — line number color
- `diff_hunk_header()` — hunk header color
