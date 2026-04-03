# stage_hunk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hunk-level staging/unstaging so pressing `s` in the right diff panel stages or unstages the focused hunk.

**Architecture:** Extend `CommandRunner` with `run_with_stdin` for piping patches. Add `format_hunk_patch` to reconstruct a single-hunk unified diff, then `stage_hunk`/`unstage_hunk` which pipe it to `git apply --cached`. Wire into the UI via `event.rs` with panel-sensitive `s` key.

**Tech Stack:** Rust, gitat-core (git operations), gitat-ui (ratatui TUI), insta (snapshot tests)

---

### Task 1: Add `run_with_stdin` to `CommandRunner` trait

**Files:**
- Modify: `crates/gitat-core/src/runner.rs`

- [ ] **Step 1: Write the failing test for `MockRunner::run_with_stdin`**

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/gitat-core/src/runner.rs`:

```rust
#[test]
fn test_mock_runner_run_with_stdin() {
    let runner = MockRunner::new()
        .with_response("apply --cached", "");
    let result = runner.run_with_stdin(&["apply", "--cached"], "patch content");
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-core test_mock_runner_run_with_stdin`
Expected: FAIL — `run_with_stdin` method not found on `MockRunner`

- [ ] **Step 3: Add `run_with_stdin` to `CommandRunner` trait and implement for `ProcessRunner` and `MockRunner`**

In `crates/gitat-core/src/runner.rs`, change the `CommandRunner` trait to:

```rust
pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
    fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, GitError>;
}
```

Add to `ProcessRunner`'s `impl CommandRunner` block:

```rust
fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, GitError> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = std::process::Command::new("git")
        .args(args)
        .current_dir(&self.repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GitError::IoError(e.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .map_err(|e| GitError::IoError(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| GitError::IoError(e.to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(GitError::CommandFailed {
            command: format!("git {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
```

Add to `MockRunner`'s `impl CommandRunner` block:

```rust
fn run_with_stdin(&self, args: &[&str], _stdin_data: &str) -> Result<String, GitError> {
    self.run(args)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-core test_mock_runner_run_with_stdin`
Expected: PASS

- [ ] **Step 5: Run all tests to verify nothing is broken**

Run: `cargo test --workspace`
Expected: All existing tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-core/src/runner.rs
git commit -m "feat: add run_with_stdin to CommandRunner trait

Add stdin piping support for commands like git apply --cached.
ProcessRunner pipes data via child process stdin.
MockRunner delegates to run() ignoring stdin content."
```

---

### Task 2: Implement `format_hunk_patch`

**Files:**
- Modify: `crates/gitat-core/src/stage.rs`

- [ ] **Step 1: Write the failing test for a normal modification hunk**

Add `use crate::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};` at the top of `stage.rs`, and add a `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};

    fn make_simple_diff_file() -> DiffFile {
        DiffFile {
            old_path: "src/main.rs".to_string(),
            new_path: "src/main.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 4,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "    println!(\"hello\");".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    let msg = \"hello\";".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    println!(\"{msg}\");".to_string(),
                        old_line_no: None,
                        new_line_no: Some(3),
                    },
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "}".to_string(),
                        old_line_no: Some(3),
                        new_line_no: Some(4),
                    },
                ],
            }],
        }
    }

    #[test]
    fn test_format_hunk_patch_normal() {
        let diff_file = make_simple_diff_file();
        let patch = format_hunk_patch(&diff_file, 0).unwrap();
        let expected = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!(\"hello\");
+    let msg = \"hello\";
+    println!(\"{msg}\");
 }
";
        assert_eq!(patch, expected);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-core test_format_hunk_patch_normal`
Expected: FAIL — `format_hunk_patch` not found

- [ ] **Step 3: Implement `format_hunk_patch`**

Add at the top of `stage.rs` (after the existing `use` statements):

```rust
use crate::diff::{DiffFile, DiffHunk, DiffLineKind};
```

Add the function (above the existing `stage_file` function):

```rust
fn format_hunk_patch(diff_file: &DiffFile, hunk_index: usize) -> Result<String, GitError> {
    let hunk = diff_file.hunks.get(hunk_index).ok_or_else(|| {
        GitError::ParseError(format!(
            "hunk index {} out of bounds (file has {} hunks)",
            hunk_index,
            diff_file.hunks.len()
        ))
    })?;

    let mut patch = String::new();

    // File header
    let old_header = if diff_file.old_path == "/dev/null" {
        "/dev/null".to_string()
    } else {
        format!("a/{}", diff_file.old_path)
    };
    let new_header = if diff_file.new_path == "/dev/null" {
        "/dev/null".to_string()
    } else {
        format!("b/{}", diff_file.new_path)
    };

    patch.push_str(&format!("diff --git a/{} b/{}\n", diff_file.old_path, diff_file.new_path));
    patch.push_str(&format!("--- {}\n", old_header));
    patch.push_str(&format!("+++ {}\n", new_header));

    // Hunk header
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
    ));

    // Hunk lines
    for line in &hunk.lines {
        match line.kind {
            DiffLineKind::Context => {
                patch.push(' ');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            DiffLineKind::Added => {
                patch.push('+');
                patch.push_str(&line.content);
                patch.push('\n');
            }
            DiffLineKind::Removed => {
                patch.push('-');
                patch.push_str(&line.content);
                patch.push('\n');
            }
        }
    }

    Ok(patch)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-core test_format_hunk_patch_normal`
Expected: PASS

- [ ] **Step 5: Write test for new file (old_path == "/dev/null")**

Add to the `tests` module:

```rust
#[test]
fn test_format_hunk_patch_new_file() {
    let diff_file = DiffFile {
        old_path: "/dev/null".to_string(),
        new_path: "new.rs".to_string(),
        hunks: vec![DiffHunk {
            old_start: 0,
            old_count: 0,
            new_start: 1,
            new_count: 2,
            lines: vec![
                DiffLine {
                    kind: DiffLineKind::Added,
                    content: "fn hello() {}".to_string(),
                    old_line_no: None,
                    new_line_no: Some(1),
                },
                DiffLine {
                    kind: DiffLineKind::Added,
                    content: "fn world() {}".to_string(),
                    old_line_no: None,
                    new_line_no: Some(2),
                },
            ],
        }],
    };
    let patch = format_hunk_patch(&diff_file, 0).unwrap();
    assert!(patch.contains("--- /dev/null\n"));
    assert!(patch.contains("+++ b/new.rs\n"));
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p gitat-core test_format_hunk_patch_new_file`
Expected: PASS (implementation already handles this case)

- [ ] **Step 7: Write test for out-of-bounds hunk index**

Add to the `tests` module:

```rust
#[test]
fn test_format_hunk_patch_out_of_bounds() {
    let diff_file = make_simple_diff_file();
    let result = format_hunk_patch(&diff_file, 5);
    assert!(result.is_err());
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test -p gitat-core test_format_hunk_patch_out_of_bounds`
Expected: PASS

- [ ] **Step 9: Write snapshot test for multi-line hunk**

Add to the `tests` module:

```rust
#[test]
fn snapshot_format_hunk_patch() {
    let diff_file = make_simple_diff_file();
    let patch = format_hunk_patch(&diff_file, 0).unwrap();
    insta::assert_snapshot!(patch);
}
```

- [ ] **Step 10: Run snapshot test and accept**

Run: `cargo test -p gitat-core snapshot_format_hunk_patch`
Then: `cargo insta review` (or manually move the `.snap.new` file to `.snap`)

If `cargo-insta` is not installed, manually copy the generated snapshot:
```bash
mv crates/gitat-core/src/snapshots/gitat_core__stage__tests__snapshot_format_hunk_patch.snap.new \
   crates/gitat-core/src/snapshots/gitat_core__stage__tests__snapshot_format_hunk_patch.snap
```

Then re-run: `cargo test -p gitat-core snapshot_format_hunk_patch`
Expected: PASS

- [ ] **Step 11: Commit**

```bash
git add crates/gitat-core/src/stage.rs crates/gitat-core/src/snapshots/
git commit -m "feat: add format_hunk_patch for unified diff patch generation

Reconstructs a single-hunk unified diff patch string from DiffFile
and hunk index. Handles normal files, new files (/dev/null old_path),
and deleted files (/dev/null new_path)."
```

---

### Task 3: Implement `stage_hunk` and `unstage_hunk`

**Files:**
- Modify: `crates/gitat-core/src/stage.rs`

- [ ] **Step 1: Write the failing test for `stage_hunk`**

Add to the `tests` module in `crates/gitat-core/src/stage.rs`:

```rust
use crate::runner::MockRunner;

#[test]
fn test_stage_hunk() {
    let diff_file = make_simple_diff_file();
    let runner = MockRunner::new()
        .with_response("apply --cached", "");
    let result = stage_hunk(&runner, &diff_file, 0);
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-core test_stage_hunk`
Expected: FAIL — `stage_hunk` not found

- [ ] **Step 3: Implement `stage_hunk` and `unstage_hunk`**

Add to `crates/gitat-core/src/stage.rs` (after `format_hunk_patch`, before `stage_file`):

```rust
pub fn stage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError> {
    let patch = format_hunk_patch(diff_file, hunk_index)?;
    runner.run_with_stdin(&["apply", "--cached"], &patch)?;
    Ok(())
}

pub fn unstage_hunk(
    runner: &dyn CommandRunner,
    diff_file: &DiffFile,
    hunk_index: usize,
) -> Result<(), GitError> {
    let patch = format_hunk_patch(diff_file, hunk_index)?;
    runner.run_with_stdin(&["apply", "--cached", "--reverse"], &patch)?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-core test_stage_hunk`
Expected: PASS

- [ ] **Step 5: Write the failing test for `unstage_hunk`**

Add to the `tests` module:

```rust
#[test]
fn test_unstage_hunk() {
    let diff_file = make_simple_diff_file();
    let runner = MockRunner::new()
        .with_response("apply --cached --reverse", "");
    let result = unstage_hunk(&runner, &diff_file, 0);
    assert!(result.is_ok());
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p gitat-core test_unstage_hunk`
Expected: PASS (already implemented in Step 3)

- [ ] **Step 7: Write test for hunk index out of bounds in `stage_hunk`**

Add to the `tests` module:

```rust
#[test]
fn test_stage_hunk_out_of_bounds() {
    let diff_file = make_simple_diff_file();
    let runner = MockRunner::new();
    let result = stage_hunk(&runner, &diff_file, 99);
    assert!(result.is_err());
}
```

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test -p gitat-core test_stage_hunk_out_of_bounds`
Expected: PASS

- [ ] **Step 9: Run all gitat-core tests**

Run: `cargo test -p gitat-core`
Expected: All PASS

- [ ] **Step 10: Commit**

```bash
git add crates/gitat-core/src/stage.rs
git commit -m "feat: add stage_hunk and unstage_hunk

stage_hunk pipes a single-hunk patch to git apply --cached.
unstage_hunk does the same with --reverse flag.
Both use format_hunk_patch to reconstruct the unified diff."
```

---

### Task 4: Wire up hunk staging in the UI event handler

**Files:**
- Modify: `crates/gitat-ui/src/event.rs`

- [ ] **Step 1: Write the failing test for `s` key in right panel**

Add to the `#[cfg(test)] mod tests` block in `crates/gitat-ui/src/event.rs`:

```rust
#[test]
fn test_s_in_right_panel_calls_stage_hunk() {
    let mut app = App::new();
    app.tab = Tab::Status;
    app.panel = Panel::Right;

    // Set up a status entry (unstaged modified file)
    app.status = vec![gitat_core::status::StatusEntry {
        path: "src/main.rs".to_string(),
        index_status: gitat_core::status::FileStatus::Unmodified,
        worktree_status: gitat_core::status::FileStatus::Modified,
    }];
    app.status_list_state.select(Some(0));

    // Set up current diff with one hunk
    app.current_diff = Some(vec![gitat_core::diff::DiffFile {
        old_path: "src/main.rs".to_string(),
        new_path: "src/main.rs".to_string(),
        hunks: vec![gitat_core::diff::DiffHunk {
            old_start: 1,
            old_count: 2,
            new_start: 1,
            new_count: 3,
            lines: vec![
                gitat_core::diff::DiffLine {
                    kind: gitat_core::diff::DiffLineKind::Context,
                    content: "line1".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                },
                gitat_core::diff::DiffLine {
                    kind: gitat_core::diff::DiffLineKind::Added,
                    content: "new_line".to_string(),
                    old_line_no: None,
                    new_line_no: Some(2),
                },
                gitat_core::diff::DiffLine {
                    kind: gitat_core::diff::DiffLineKind::Context,
                    content: "line2".to_string(),
                    old_line_no: Some(2),
                    new_line_no: Some(3),
                },
            ],
        }],
    }]);
    app.diff_state.current_hunk = 0;

    // MockRunner: apply --cached succeeds, then refresh commands
    let runner = MockRunner::new()
        .with_response("apply --cached", "")
        .with_response("status --porcelain=v1", "")
        .with_response("branch -a --format=%(refname:short)\t%(objectname:short)\t%(upstream:short)\t%(HEAD)", "")
        .with_response("log --pretty=format:%H\t%h\t%an\t%ai\t%s\t%D\t%P -n 100", "")
        .with_response("diff -- src/main.rs", "");

    handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);

    // After staging the only hunk, diff should be reloaded (now empty)
    assert!(app.status_message.is_none() || !app.status_message.as_ref().unwrap().contains("failed"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitat-ui test_s_in_right_panel_calls_stage_hunk`
Expected: FAIL — the current `s` handler only calls `stage_or_unstage` (file-level) regardless of panel

- [ ] **Step 3: Implement `stage_or_unstage_hunk` and modify `s` key handler**

In `crates/gitat-ui/src/event.rs`, change the `'s'` key handler in `handle_normal` from:

```rust
KeyCode::Char('s') => {
    if app.tab == Tab::Status {
        stage_or_unstage(app, runner);
    }
}
```

to:

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

Then add the `stage_or_unstage_hunk` function (after the existing `stage_or_unstage` function):

```rust
fn stage_or_unstage_hunk(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let diff_files = match &app.current_diff {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };

    let diff_file = match diff_files.iter().find(|f| f.new_path == entry.path || f.old_path == entry.path) {
        Some(f) => f.clone(),
        None => return,
    };

    let hunk_index = app.diff_state.current_hunk;

    use gitat_core::status::FileStatus;
    let is_staged = !matches!(
        entry.index_status,
        FileStatus::Unmodified | FileStatus::Untracked
    );

    let result = if is_staged {
        gitat_core::stage::unstage_hunk(runner, &diff_file, hunk_index)
    } else {
        gitat_core::stage::stage_hunk(runner, &diff_file, hunk_index)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
            // Reload diff for the same file
            let staged_after = is_staged; // If we unstaged, check unstaged diff; if staged, check staged
            let reload_staged = !staged_after; // After staging a hunk, the remaining unstaged diff
            match gitat_core::diff::get_diff_for_file(runner, &entry.path, reload_staged) {
                Ok(diff) => {
                    if diff.is_empty() || diff.iter().all(|f| f.hunks.is_empty()) {
                        app.current_diff = None;
                        app.diff_state = crate::widgets::side_by_side_diff::SideBySideDiffState::new();
                    } else {
                        // Clamp current_hunk
                        let total_hunks: usize = diff.iter().map(|f| f.hunks.len()).sum();
                        if app.diff_state.current_hunk >= total_hunks {
                            app.diff_state.current_hunk = total_hunks.saturating_sub(1);
                        }
                        app.current_diff = Some(diff);
                    }
                }
                Err(_) => {
                    app.current_diff = None;
                    app.diff_state = crate::widgets::side_by_side_diff::SideBySideDiffState::new();
                }
            }
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage hunk failed: {e}"));
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_s_in_right_panel_calls_stage_hunk`
Expected: PASS

- [ ] **Step 5: Write test verifying `s` in left panel still does file-level staging**

Add to the `tests` module:

```rust
#[test]
fn test_s_in_left_panel_still_stages_file() {
    let mut app = App::new();
    app.tab = Tab::Status;
    app.panel = Panel::Left;
    app.status = vec![gitat_core::status::StatusEntry {
        path: "src/main.rs".to_string(),
        index_status: gitat_core::status::FileStatus::Unmodified,
        worktree_status: gitat_core::status::FileStatus::Modified,
    }];
    app.status_list_state.select(Some(0));

    let runner = MockRunner::new()
        .with_response("add -- src/main.rs", "")
        .with_response("status --porcelain=v1", "")
        .with_response("branch -a --format=%(refname:short)\t%(objectname:short)\t%(upstream:short)\t%(HEAD)", "")
        .with_response("log --pretty=format:%H\t%h\t%an\t%ai\t%s\t%D\t%P -n 100", "");

    handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);
    // Should not error — file-level stage_file was called
    assert!(app.status_message.is_none());
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p gitat-ui test_s_in_left_panel_still_stages_file`
Expected: PASS

- [ ] **Step 7: Run all workspace tests**

Run: `cargo test --workspace`
Expected: All PASS

- [ ] **Step 8: Commit**

```bash
git add crates/gitat-ui/src/event.rs
git commit -m "feat: wire up hunk-level staging via s key in right panel

In Status tab, s key now stages/unstages at hunk level when the
right (diff) panel is focused, and at file level when the left
(file list) panel is focused. After hunk operation, diff is reloaded
and current_hunk index is clamped."
```

---

### Task 5: Update TODO.md

**Files:**
- Modify: `TODO.md`

- [ ] **Step 1: Mark `stage_hunk` as completed in TODO.md**

Change the line:

```
- [ ] `stage_hunk` — スペックに記載があるが `stage_file`/`unstage_file` のみ実装
```

to:

```
- [x] `stage_hunk` — スペックに記載があるが `stage_file`/`unstage_file` のみ実装
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "docs: mark stage_hunk as completed in TODO.md"
```
