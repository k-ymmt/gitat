# Unified (1-Column) Diff Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 2-column side-by-side diff widget with a GitHub PR-style 1-column unified diff viewer.

**Architecture:** Rename and refactor the existing `SideBySideDiff` widget in-place. The state management, line pairing, and word-level diff logic are preserved. Only the rendering method changes from drawing two columns to drawing a single column where each `DiffRow` expands to 1-2 screen rows (removed line then added line).

**Tech Stack:** Rust, ratatui 0.30.x, similar (word-level diff), unicode-width (CJK), insta (snapshot testing)

---

### Task 1: Rename file and update module declaration

**Files:**
- Rename: `crates/gitat-ui/src/widgets/side_by_side_diff.rs` → `crates/gitat-ui/src/widgets/unified_diff.rs`
- Modify: `crates/gitat-ui/src/widgets/mod.rs`

- [ ] **Step 1: Rename the file**

```bash
cd /Users/kazukiyamamoto/ghq/github.com/k-ymmt/gitat
git mv crates/gitat-ui/src/widgets/side_by_side_diff.rs crates/gitat-ui/src/widgets/unified_diff.rs
```

- [ ] **Step 2: Update module declaration**

In `crates/gitat-ui/src/widgets/mod.rs`, change:

```rust
pub mod side_by_side_diff;
```

to:

```rust
pub mod unified_diff;
```

- [ ] **Step 3: Verify it compiles (expect errors from callsites)**

```bash
cargo check -p gitat-ui 2>&1 | head -30
```

Expected: Compilation errors about `side_by_side_diff` not found in imports at `app.rs`, `log.rs`, `commit_detail.rs`, `uncommitted_detail.rs`, `staging.rs`. This confirms the rename worked.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: rename side_by_side_diff to unified_diff module"
```

---

### Task 2: Rename types inside unified_diff.rs

**Files:**
- Modify: `crates/gitat-ui/src/widgets/unified_diff.rs`

- [ ] **Step 1: Rename `SideBySideDiffState` to `UnifiedDiffState`**

In `crates/gitat-ui/src/widgets/unified_diff.rs`, find-and-replace all occurrences:

- `SideBySideDiffState` → `UnifiedDiffState`
- `SideBySideDiff` → `UnifiedDiff` (the widget struct and its impl blocks)

The doc comment on `DiffRow` should also be updated:

```rust
/// A paired row in the unified diff display
struct DiffRow {
```

- [ ] **Step 2: Update all tests in the same file**

All test functions in `mod tests` reference `SideBySideDiffState::new()` and `SideBySideDiff::new(...)`. These are all updated by the find-and-replace in step 1. Verify the following are renamed:

- `SideBySideDiffState::new()` → `UnifiedDiffState::new()`
- `SideBySideDiff::new(&diff)` → `UnifiedDiff::new(&diff)`

- [ ] **Step 3: Commit**

```bash
git add crates/gitat-ui/src/widgets/unified_diff.rs
git commit -m "refactor: rename SideBySideDiff to UnifiedDiff types"
```

---

### Task 3: Update all callsites to use new module and type names

**Files:**
- Modify: `crates/gitat-ui/src/app.rs:13,73,87,101,115`
- Modify: `crates/gitat-ui/src/views/log.rs:9,334,500`
- Modify: `crates/gitat-ui/src/event/commit_detail.rs:4,48,79,93`
- Modify: `crates/gitat-ui/src/event/uncommitted_detail.rs:4,15,39,56`
- Modify: `crates/gitat-ui/src/event/staging.rs:2,75,88`

- [ ] **Step 1: Update app.rs**

Change the import:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiffState;
```

to:

```rust
use crate::widgets::unified_diff::UnifiedDiffState;
```

Change all occurrences of `SideBySideDiffState` to `UnifiedDiffState` in the file (field types and `::new()` calls).

- [ ] **Step 2: Update views/log.rs**

Change the import:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiff;
```

to:

```rust
use crate::widgets::unified_diff::UnifiedDiff;
```

Change both widget construction calls from `SideBySideDiff::new(...)` to `UnifiedDiff::new(...)` (lines 334 and 500).

- [ ] **Step 3: Update event/commit_detail.rs**

Change the import:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiffState;
```

to:

```rust
use crate::widgets::unified_diff::UnifiedDiffState;
```

Change all `SideBySideDiffState::new()` to `UnifiedDiffState::new()` (lines 48, 79, 93).

- [ ] **Step 4: Update event/uncommitted_detail.rs**

Change the import:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiffState;
```

to:

```rust
use crate::widgets::unified_diff::UnifiedDiffState;
```

Change all `SideBySideDiffState::new()` to `UnifiedDiffState::new()` (lines 15, 39, 56).

- [ ] **Step 5: Update event/staging.rs**

Change the import:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiffState;
```

to:

```rust
use crate::widgets::unified_diff::UnifiedDiffState;
```

Change all `SideBySideDiffState::new()` to `UnifiedDiffState::new()` (lines 75, 88).

- [ ] **Step 6: Verify compilation**

```bash
cargo check -p gitat-ui
```

Expected: Compiles successfully with no errors.

- [ ] **Step 7: Run existing tests**

```bash
cargo test -p gitat-ui
```

Expected: All tests pass (snapshots still match the old 2-column rendering since we haven't changed the render method yet).

- [ ] **Step 8: Commit**

```bash
git add crates/gitat-ui/src/app.rs crates/gitat-ui/src/views/log.rs crates/gitat-ui/src/event/commit_detail.rs crates/gitat-ui/src/event/uncommitted_detail.rs crates/gitat-ui/src/event/staging.rs
git commit -m "refactor: update all callsites to use UnifiedDiff types"
```

---

### Task 4: Refactor render method — convert from 2-column to 1-column layout

This is the core change. The `render` method in `impl StatefulWidget for UnifiedDiff` needs to be rewritten to draw a single column instead of two.

**Files:**
- Modify: `crates/gitat-ui/src/widgets/unified_diff.rs` (the `render` method, lines 308-462)

- [ ] **Step 1: Delete old snapshot files**

The old snapshots reference the old module name `side_by_side_diff` in their filenames. Delete them so `insta` generates fresh ones:

```bash
rm crates/gitat-ui/src/widgets/snapshots/gitat_ui__widgets__side_by_side_diff__tests__*.snap
```

- [ ] **Step 2: Refactor the `render_content_with_word_diff` function**

The `is_left` parameter concept no longer applies in unified mode. In 1-column, removed lines always show Delete highlights and added lines always show Insert highlights. Replace the function with:

```rust
/// Renders content with word-level diff highlighting for unified display.
///
/// When `paired_content` is provided, uses `similar::TextDiff::from_words` to
/// compute word-level changes. For Removed lines, highlights deleted words.
/// For Added lines, highlights inserted words.
fn render_content_with_word_diff(
    content: &str,
    paired_content: Option<&str>,
    kind: &DiffLineKind,
) -> Vec<StyledSegment> {
    let base_style = match kind {
        DiffLineKind::Added => Theme::diff_added(),
        DiffLineKind::Removed => Theme::diff_removed(),
        DiffLineKind::Context => Theme::diff_context(),
    };

    let paired = match paired_content {
        Some(p) => p,
        None => {
            return vec![StyledSegment {
                text: content.to_string(),
                style: base_style,
            }];
        }
    };

    if matches!(kind, DiffLineKind::Context) {
        return vec![StyledSegment {
            text: content.to_string(),
            style: base_style,
        }];
    }

    let is_removed = matches!(kind, DiffLineKind::Removed);
    let (old, new) = if is_removed {
        (content, paired)
    } else {
        (paired, content)
    };

    let text_diff = TextDiff::from_words(old, new);
    let mut segments = Vec::new();

    for change in text_diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                segments.push(StyledSegment {
                    text: change.value().to_string(),
                    style: base_style,
                });
            }
            ChangeTag::Delete => {
                if is_removed {
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: Theme::diff_word_removed(),
                    });
                }
            }
            ChangeTag::Insert => {
                if !is_removed {
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: Theme::diff_word_added(),
                    });
                }
            }
        }
    }

    segments
}
```

- [ ] **Step 3: Rewrite the `render` method**

Replace the entire `render` method body in `impl StatefulWidget for UnifiedDiff<'_>` with:

```rust
fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
    // Handle block
    let inner = if let Some(block) = self.block {
        let inner = block.inner(area);
        block.render(area, buf);
        inner
    } else {
        area
    };

    // Early return if area too small
    if inner.width < 12 || inner.height < 2 {
        return;
    }

    // Build all DiffRows from all files/hunks, and expand to screen rows.
    // Each DiffRow may produce 1 row (context or unpaired change) or
    // 2 rows (paired removed + added).
    struct ScreenRow {
        line_no_old: Option<u32>,
        line_no_new: Option<u32>,
        content: String,
        kind: DiffLineKind,
        paired_content: Option<String>,
    }

    let mut screen_rows: Vec<ScreenRow> = Vec::new();
    let mut hunk_offsets: Vec<u16> = Vec::new();

    for file in self.diff_files {
        for hunk in &file.hunks {
            hunk_offsets.push(screen_rows.len() as u16);
            let diff_rows = pair_lines(hunk);
            for row in diff_rows {
                let has_left = row.left_content.is_some()
                    && !matches!(row.left_kind, DiffLineKind::Context);
                let has_right = row.right_content.is_some()
                    && !matches!(row.right_kind, DiffLineKind::Context);
                let is_paired = has_left && has_right;

                if matches!(row.left_kind, DiffLineKind::Context) {
                    // Context line: single row
                    screen_rows.push(ScreenRow {
                        line_no_old: row.left_line_no,
                        line_no_new: row.right_line_no,
                        content: row.left_content.unwrap_or_default(),
                        kind: DiffLineKind::Context,
                        paired_content: None,
                    });
                } else {
                    // Removed line (if present)
                    if let Some(ref left_content) = row.left_content {
                        screen_rows.push(ScreenRow {
                            line_no_old: row.left_line_no,
                            line_no_new: None,
                            content: left_content.clone(),
                            kind: DiffLineKind::Removed,
                            paired_content: if is_paired {
                                row.right_content.clone()
                            } else {
                                None
                            },
                        });
                    }
                    // Added line (if present)
                    if let Some(ref right_content) = row.right_content {
                        screen_rows.push(ScreenRow {
                            line_no_old: None,
                            line_no_new: row.right_line_no,
                            content: right_content.clone(),
                            kind: DiffLineKind::Added,
                            paired_content: if is_paired {
                                row.left_content.clone()
                            } else {
                                None
                            },
                        });
                    }
                }
            }
        }
    }

    state.hunk_offsets = hunk_offsets;

    // Layout: [OldLN(4)] [NewLN(4)] [sep(1)] [Content]
    let line_no_width: u16 = 4;
    let separator_width: u16 = 1;
    let content_x = inner.x + line_no_width + line_no_width + separator_width;
    let content_width = inner
        .width
        .saturating_sub(line_no_width + line_no_width + separator_width);

    for row_idx in 0..inner.height {
        let data_idx = state.scroll_y as usize + row_idx as usize;
        let y = inner.y + row_idx;

        if data_idx >= screen_rows.len() {
            break;
        }

        let row = &screen_rows[data_idx];

        // Old line number
        let old_ln_str = match row.line_no_old {
            Some(n) => format!("{:>width$}", n, width = line_no_width as usize),
            None => " ".repeat(line_no_width as usize),
        };
        buf.set_string(inner.x, y, &old_ln_str, Theme::diff_line_number());

        // New line number
        let new_ln_str = match row.line_no_new {
            Some(n) => format!("{:>width$}", n, width = line_no_width as usize),
            None => " ".repeat(line_no_width as usize),
        };
        buf.set_string(
            inner.x + line_no_width,
            y,
            &new_ln_str,
            Theme::diff_line_number(),
        );

        // Separator
        let sep_x = inner.x + line_no_width + line_no_width;
        buf.set_string(sep_x, y, "\u{2502}", Theme::diff_line_number());

        // Content
        let bg_style = match row.kind {
            DiffLineKind::Added => Theme::diff_added(),
            DiffLineKind::Removed => Theme::diff_removed(),
            DiffLineKind::Context => Theme::diff_context(),
        };

        let segments = render_content_with_word_diff(
            &row.content,
            row.paired_content.as_deref(),
            &row.kind,
        );
        write_segments(
            buf,
            &segments,
            content_x,
            y,
            content_width,
            state.scroll_x,
            bg_style,
        );
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p gitat-ui
```

Expected: Compiles successfully.

- [ ] **Step 5: Run tests to generate new snapshots**

```bash
cargo test -p gitat-ui -- unified_diff 2>&1
```

Expected: All snapshot tests fail with "new snapshot" — no existing snapshots match the new module path. This is expected.

- [ ] **Step 6: Review and accept new snapshots**

```bash
cargo insta review
```

Verify each snapshot shows the 1-column layout:
- Two line number columns (old, new) on the left
- Separator (`│`)
- Full-width content on the right
- Removed lines show old line number only, added lines show new line number only
- Context lines show both line numbers

Accept all snapshots.

- [ ] **Step 7: Run all tests to confirm green**

```bash
cargo test -p gitat-ui
```

Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: convert diff viewer from 2-column side-by-side to 1-column unified layout"
```

---

### Task 5: Full build and integration verification

**Files:** None (verification only)

- [ ] **Step 1: Build the full workspace**

```bash
cargo build
```

Expected: Builds successfully with no errors or warnings.

- [ ] **Step 2: Run all tests across all crates**

```bash
cargo test
```

Expected: All tests pass across `gitat-core`, `gitat-ui`, and `gitat`.

- [ ] **Step 3: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings or errors.

- [ ] **Step 4: Commit any clippy fixes if needed**

If clippy finds issues, fix and commit:

```bash
git add -A
git commit -m "fix: address clippy warnings in unified diff widget"
```
