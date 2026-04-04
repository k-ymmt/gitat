# Search Mode Improvements Design

## Overview

Enhance the search mode in the Log tab with cursor navigation, preview synchronization, and direct commit detail access. Introduces a mode stack for clean mode transitions.

## Current State

- Search mode is entered via `/` in Normal mode
- Typing filters `log_entries` by message, hash, or author (case-insensitive)
- Enter: clears filter, moves cursor to selected commit, returns to Normal
- Esc: clears filter, restores cursor to pre-search position, returns to Normal
- No j/k navigation within search results
- No preview panel updates during search
- No way to enter CommitDetail directly from search

## Requirements

1. **j/k navigation in search results** — move cursor up/down within filtered results while in Search mode
2. **Preview synchronization** — update the preview panel (right side) when cursor moves in filtered results
3. **Direct CommitDetail from search** — Enter opens CommitDetail for the selected commit; exiting CommitDetail returns to Search mode with filter and query preserved
4. **Esc behavior unchanged** — Esc clears filter, restores cursor, returns to Normal

## Design

### Mode Stack

Add `mode_stack: Vec<Mode>` to `App` with two helper methods:

```rust
pub mode_stack: Vec<Mode>,

pub fn push_mode(&mut self, mode: Mode) {
    let current = std::mem::replace(&mut self.mode, mode);
    self.mode_stack.push(current);
}

pub fn pop_mode(&mut self) {
    if let Some(prev) = self.mode_stack.pop() {
        self.mode = prev;
    }
}
```

- `push_mode`: saves current mode to stack, transitions to new mode
- `pop_mode`: restores previous mode from stack (no-op if stack is empty)

**Scope**: Only Search -> CommitDetail uses push/pop in this change. Other existing mode transitions (Normal -> Commit, Normal -> Help, etc.) remain as direct assignment. No bulk migration.

### Search Event Handler Changes

Add j/k key handling and modify Enter behavior. Requires adding `runner: &dyn CommandRunner` parameter to `handle_search`.

**j/k navigation:**

```rust
KeyCode::Char('j') => {
    let max = app.filtered_log_indices.as_ref().map_or(0, |v| v.len());
    if max > 0 {
        let current = app.log_list_state.selected().unwrap_or(0);
        let next = (current + 1).min(max - 1);
        app.log_list_state.select(Some(next));
    }
    load_log_preview(app, runner);
}
KeyCode::Char('k') => {
    let current = app.log_list_state.selected().unwrap_or(0);
    app.log_list_state.select(Some(current.saturating_sub(1)));
    load_log_preview(app, runner);
}
```

- Cursor movement is clamped to filtered result bounds
- `load_log_preview` is called after each move to update the preview panel
- The existing `load_log_preview` already handles filtered mode correctly (treats all items as commits, no uncommitted row offset)

**Enter — direct CommitDetail transition:**

```rust
KeyCode::Enter => {
    let original_index = app.log_list_state.selected().and_then(|sel| {
        app.filtered_log_indices
            .as_ref()
            .and_then(|indices| indices.get(sel).copied())
    });
    if let Some(idx) = original_index {
        // Load commit detail data (reuse enter_commit_detail logic)
        // push_mode: saves Search to stack, transitions to CommitDetail
        app.push_mode(Mode::CommitDetail);
    }
}
```

- `filtered_log_indices` and `pre_search_cursor` are NOT cleared (preserved for return to Search)
- If no selection or empty results, Enter is a no-op

**Esc — unchanged except stack cleanup:**

```rust
KeyCode::Esc => {
    if let Some(cursor) = app.pre_search_cursor {
        app.log_list_state.select(Some(cursor));
    }
    app.filtered_log_indices = None;
    app.pre_search_cursor = None;
    app.mode = Mode::Normal;
    app.mode_stack.clear();
}
```

- `mode_stack.clear()` ensures no stale entries remain

### CommitDetail Exit Change

Replace `app.mode = Mode::Normal` with `app.pop_mode()` in the CommitDetail event handler's q/Esc handling. This way:

- When entered from Search: pops back to Search (filter and query preserved)
- When entered from Normal (existing behavior): stack is empty, `pop_mode` is a no-op — need to handle this by falling back to `Mode::Normal`

Adjusted `pop_mode` or the exit handler should ensure a fallback:

```rust
// In CommitDetail handler
KeyCode::Char('q') | KeyCode::Esc => {
    app.pop_mode();
    // If stack was empty, pop_mode is no-op, so ensure we go to Normal
    if matches!(app.mode, Mode::CommitDetail) {
        app.mode = Mode::Normal;
    }
}
```

### Data Flow Summary

```
Normal --(/)--> Search --j/k--> Search (cursor moves, preview updates)
                       --Enter-> CommitDetail (push_mode, filter preserved)
                       --Esc---> Normal (filter cleared, cursor restored)

CommitDetail --q/Esc-> pop_mode() -> Search (if entered from Search)
                                  -> Normal (if entered from Normal)
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/gitat-ui/src/app.rs` | Add `mode_stack` field, `push_mode`, `pop_mode` methods |
| `crates/gitat-ui/src/event/mod.rs` | Add `runner` param to `handle_search`, update call site |
| `crates/gitat-ui/src/event/mod.rs` | Add j/k handling with `load_log_preview` calls in `handle_search` |
| `crates/gitat-ui/src/event/mod.rs` | Change Enter to `push_mode(CommitDetail)` with commit data loading |
| `crates/gitat-ui/src/event/mod.rs` | Add `mode_stack.clear()` to Esc handler |
| `crates/gitat-ui/src/event/commit_detail.rs` | Change exit to `pop_mode()` with Normal fallback |

## Testing

- Snapshot tests for filtered list rendering with different cursor positions
- Unit tests for `push_mode`/`pop_mode` behavior
- Test Search -> Enter -> CommitDetail -> q -> returns to Search with filter preserved
- Test Search -> Esc -> Normal with filter cleared and cursor restored
- Test j/k navigation bounds (first item, last item, empty results)
- Test preview loading is triggered on j/k movement
