# stage_hunk — Hunk-level Staging/Unstaging Design

## Overview

Add hunk-level staging and unstaging to gitat. When the user is viewing a diff in the right panel and presses `s`, the currently focused hunk is staged (or unstaged if viewing a staged diff). This mirrors the lazygit pattern where `s` is context-sensitive based on the active panel.

## Motivation

File-level staging (`stage_file` / `unstage_file`) is already implemented, but hunk-level staging is one of the core reasons users prefer a TUI git client over the command line. It enables selective staging of changes within a single file, equivalent to `git add -p` but with a visual diff.

## Scope

### In Scope

- `stage_hunk`: stage a single hunk from the unstaged diff
- `unstage_hunk`: unstage a single hunk from the staged diff
- Key binding: `s` in the right panel (diff view) on the Status tab
- Diff reload after hunk operation
- Hunk index clamping after reload

### Out of Scope

- Line-level staging (`stage_line`)
- Hunk staging in commit detail view (read-only, past commits)
- Hunk staging for new/untracked files (these must be staged as whole files)

## Design

### 1. `CommandRunner` — Add `run_with_stdin`

`git apply --cached` requires patch data via stdin. The current `CommandRunner` trait only supports argument-based invocation.

**Change:** Add `run_with_stdin` method to the `CommandRunner` trait.

```rust
pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
    fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Result<String, GitError>;
}
```

**`ProcessRunner`:** Pipe `stdin` string to the child process's stdin via `Command::stdin(Stdio::piped())` and `child.stdin.write_all()`.

**`MockRunner`:** Store responses keyed by args (same as `run`). The stdin content is not included in the key — tests set up expected args and the corresponding response. A dedicated `with_stdin_response` builder method can be added if needed, but initially keying by args alone is sufficient since `git apply` commands will have unique arg combinations.

### 2. `format_hunk_patch` — Reconstruct Unified Diff Patch

Add a function in `stage.rs` that takes a `DiffFile` and a hunk index, and produces a minimal unified diff patch string that `git apply` can parse.

```rust
fn format_hunk_patch(diff_file: &DiffFile, hunk_index: usize) -> Result<String, GitError>
```

Output format:

```
diff --git a/{old_path} b/{new_path}
--- a/{old_path}
+++ b/{new_path}
@@ -{old_start},{old_count} +{new_start},{new_count} @@
 context line
-removed line
+added line
```

Special cases:
- New files (`old_path == "/dev/null"`): use `--- /dev/null` without `a/` prefix
- Deleted files (`new_path == "/dev/null"`): use `+++ /dev/null` without `b/` prefix

### 3. `stage_hunk` / `unstage_hunk` Functions

```rust
pub fn stage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError>;

pub fn unstage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError>;
```

- `stage_hunk`: generates patch via `format_hunk_patch`, pipes to `git apply --cached`
- `unstage_hunk`: generates patch via `format_hunk_patch`, pipes to `git apply --cached --reverse`

Both return `GitError` on failure (e.g., patch doesn't apply cleanly).

### 4. Event Handling Changes (`event.rs`)

Modify `handle_normal`'s `KeyCode::Char('s')` branch:

```rust
KeyCode::Char('s') => {
    if app.tab == Tab::Status {
        match app.panel {
            Panel::Left => stage_or_unstage(app, runner),
            Panel::Right => stage_or_unstage_hunk(app, runner),
        }
    }
}
```

**`stage_or_unstage_hunk` logic:**

1. Get the currently selected file from `status_list_state`
2. Determine if the file is staged (same logic as `stage_or_unstage`)
3. Get the current diff from `app.current_diff`
4. Find the `DiffFile` matching the selected file
5. Use `app.diff_state.current_hunk` as the hunk index
6. Call `stage_hunk` or `unstage_hunk` accordingly
7. On success: refresh status, reload diff for the file
8. Clamp `current_hunk` if the hunk count decreased after reload
9. Show status message on error

### 5. Post-Operation State Management

After a hunk is staged/unstaged:
- `app.refresh(runner)` to update the status list
- Reload the diff for the same file via `get_diff_for_file`
- If the file no longer has unstaged changes (all hunks staged), the diff will be empty — clear `current_diff`
- Clamp `diff_state.current_hunk` to `max(0, new_hunk_count - 1)`

## Testing Strategy

### Unit Tests (gitat-core)

- `format_hunk_patch`: verify output format for normal changes, new files, deleted files, multi-line hunks
- `stage_hunk` / `unstage_hunk`: mock runner verifies correct args (`apply --cached` / `apply --cached --reverse`) are called with patch on stdin
- `run_with_stdin` on `ProcessRunner`: integration test using a real git repo (optional, can be skipped for CI speed)

### Unit Tests (gitat-ui)

- `stage_or_unstage_hunk`: mock runner + pre-populated `App` state, verify correct core function is called and state is updated
- Key binding test: press `s` with `Panel::Right` and `Tab::Status`, verify hunk staging is triggered

## Error Handling

- Hunk index out of bounds: return early with no-op (defensive, should not happen in practice)
- `git apply` failure: display error in status message bar (e.g., "Stage hunk failed: {stderr}")
- No diff loaded: return early with no-op
