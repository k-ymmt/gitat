# Event Module Refactoring Design

## Overview

Refactor `gitat-ui/src/event.rs` (730 lines) to improve maintainability by:
1. Extracting duplicated `is_staged` logic into a method on `StatusEntry`
2. Splitting `event.rs` into focused submodules
3. Adding user-facing error notification for diff reload failures

## 1. `is_staged` Helper Extraction

### Problem

The same `is_staged` pattern appears in 3 places in `event.rs`:
- `stage_or_unstage` (line 265-268)
- `stage_or_unstage_hunk` (line 308-311)
- `load_diff_for_selected` (line 388-391)

```rust
use gitat_core::status::FileStatus;
let is_staged = !matches!(
    entry.index_status,
    FileStatus::Unmodified | FileStatus::Untracked
);
```

### Solution

Add `is_staged()` method to `StatusEntry` in `gitat-core/src/status.rs`:

```rust
impl StatusEntry {
    pub fn is_staged(&self) -> bool {
        !matches!(
            self.index_status,
            FileStatus::Unmodified | FileStatus::Untracked
        )
    }
}
```

Replace all 3 occurrences with `entry.is_staged()`. Remove the now-unnecessary `use gitat_core::status::FileStatus;` imports at each call site.

## 2. `event.rs` Module Split

### Current State

`event.rs` contains 7 mode handlers, staging operations, diff loading, branch operations, and tests — all in a single 730-line file.

### Target Structure

```
event/
  mod.rs           — handle_key dispatch + small handlers (commit, help, search, conflict)
  normal.rs        — handle_normal + list_len
  staging.rs       — stage_or_unstage, stage_or_unstage_hunk
  commit_detail.rs — handle_commit_detail + enter_commit_detail + load_commit_detail_diff
```

### Module Details

#### `event/mod.rs` (~200 lines)

Public API:
- `pub fn handle_key(app, key, runner)` — top-level dispatch

Internal handlers kept here (too small to split):
- `fn handle_commit(app, key, runner)` (~40 lines)
- `fn handle_help(app, key)` (~8 lines)
- `fn handle_search(app, key)` (~18 lines)
- `fn handle_conflict(app, key, runner)` (~35 lines)

Submodule declarations:
- `mod normal;`
- `mod staging;`
- `mod commit_detail;`

Tests: Integration tests that exercise `handle_key` dispatch (tab switch, quit, enter commit mode, etc.) stay in `mod.rs`.

#### `event/normal.rs` (~140 lines)

- `pub(super) fn handle_normal(app, key, runner)` — normal mode key handler
- `fn list_len(app)` — helper for j/k navigation

Calls into `staging::stage_or_unstage`, `staging::stage_or_unstage_hunk`, `commit_detail::enter_commit_detail` via `super::` paths.

No tests needed here — `handle_normal` is tested through `handle_key` in `mod.rs`.

#### `event/staging.rs` (~100 lines)

- `pub(super) fn stage_or_unstage(app, runner)`
- `pub(super) fn stage_or_unstage_hunk(app, runner)`
- `pub(super) fn load_diff_for_selected(app, runner)` — also staging-adjacent (determines staged/unstaged for diff loading)

Tests: staging-specific tests (`test_s_in_right_panel_calls_stage_hunk`, `test_s_in_left_panel_still_stages_file`) move here.

#### `event/commit_detail.rs` (~120 lines)

- `pub(super) fn enter_commit_detail(app, runner)`
- `pub(super) fn handle_commit_detail(app, key, runner)`
- `fn load_commit_detail_diff(app, runner)` — private to this module

Tests: commit detail tests (`test_enter_log_tab_enters_commit_detail_mode`, `test_esc_from_commit_detail_returns_to_normal`, `test_commit_detail_panel_switch`) move here.

### Visibility

All inter-module functions use `pub(super)` to keep them internal to the `event` module. The only public function is `handle_key` in `mod.rs`.

## 3. Diff Reload Failure Notification

### Problem

In `stage_or_unstage_hunk`, when the diff reload after staging fails (line 338), the error is silently swallowed:

```rust
Err(_) => {
    app.current_diff = None;
    app.diff_state = SideBySideDiffState::new();
}
```

### Solution

Notify the user via status message:

```rust
Err(e) => {
    app.set_status_message(format!("Failed to reload diff: {e}"));
    app.current_diff = None;
    app.diff_state = SideBySideDiffState::new();
}
```

## Implementation Order

1. Add `is_staged()` to `StatusEntry` — standalone change, easy to verify
2. Split `event.rs` into `event/` module — mechanical restructuring, no logic changes
3. Add error notification in `stage_or_unstage_hunk` — single-line change
4. Run tests to confirm no regressions
5. Update `TODO.md` to mark items as completed
