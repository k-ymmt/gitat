# gitat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a lazygit-style Git TUI client with high-quality side-by-side diff and conflict resolution.

**Architecture:** Rust workspace with 3 crates — `gitat-core` (git CLI wrapper with parsed types), `gitat-ui` (ratatui TUI layer), `gitat` (binary entry point). Core has no TUI dependency. Git operations use `std::process::Command`.

**Tech Stack:** Rust 2024 edition, ratatui 0.30.0, crossterm, thiserror, anyhow, similar, insta

**Spec:** `docs/superpowers/specs/2026-04-03-gitat-design.md`

**Skill:** Use `@ratatui` skill when implementing any ratatui widget, layout, or TUI pattern.

**Spec deviations:**
- `repository.rs` from spec is subsumed by `runner.rs` (ProcessRunner holds repo_path)
- `views/diff.rs` and `views/conflict.rs` from spec are inlined — diff rendering lives in `views/status.rs` detail panel, conflict rendering is dispatched from `main.rs` via the `ConflictEditor` widget directly
- `GitError::IoError` stores `String` instead of `std::io::Error` to support `Clone` (needed by MockRunner)

---

## File Structure

```
gitat/
├── Cargo.toml                          # workspace root
├── crates/
│   ├── gitat-core/
│   │   ├── Cargo.toml                  # thiserror dep
│   │   └── src/
│   │       ├── lib.rs                  # re-exports, CommandRunner trait, GitError
│   │       ├── runner.rs               # ProcessRunner, MockRunner
│   │       ├── status.rs               # StatusEntry, FileStatus, parse_status()
│   │       ├── log.rs                  # CommitInfo, parse_log()
│   │       ├── diff.rs                 # DiffFile, DiffHunk, DiffLine, parse_diff()
│   │       ├── branch.rs              # BranchInfo, branch operations
│   │       ├── stage.rs               # stage/unstage operations
│   │       ├── commit.rs              # commit operations
│   │       ├── remote.rs              # push/pull/fetch
│   │       └── conflict.rs            # ConflictFile, ConflictRegion, parse/resolve
│   │
│   ├── gitat-ui/
│   │   ├── Cargo.toml                  # ratatui, crossterm, gitat-core deps
│   │   └── src/
│   │       ├── lib.rs                  # re-exports
│   │       ├── app.rs                  # App struct, Tab, Mode, state management
│   │       ├── event.rs               # handle_key(), keybinding dispatch
│   │       ├── theme.rs               # color constants, Style helpers
│   │       ├── widgets/
│   │       │   ├── mod.rs
│   │       │   ├── side_by_side_diff.rs  # SideBySideDiff StatefulWidget
│   │       │   └── conflict_editor.rs    # ConflictEditor StatefulWidget
│   │       └── views/
│   │           ├── mod.rs
│   │           ├── status.rs           # Status tab: file list + diff detail
│   │           ├── log.rs              # Log tab: commit list + detail
│   │           ├── branches.rs         # Branches tab: branch list + actions
│   │           └── commit.rs           # Commit mode popup
│   │
│   └── gitat/
│       ├── Cargo.toml                  # gitat-ui, gitat-core, anyhow, crossterm deps
│       └── src/
│           └── main.rs                 # entry point, terminal setup, event loop
```

---

## Task 1: Workspace Setup

Convert the current flat project into a Cargo workspace with 3 crates.

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/gitat-core/Cargo.toml`
- Create: `crates/gitat-core/src/lib.rs`
- Create: `crates/gitat-ui/Cargo.toml`
- Create: `crates/gitat-ui/src/lib.rs`
- Create: `crates/gitat/Cargo.toml`
- Create: `crates/gitat/src/main.rs`
- Delete: `src/main.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

Replace `Cargo.toml` with workspace definition:

```toml
[workspace]
members = ["crates/*"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.dependencies]
gitat-core = { path = "crates/gitat-core" }
gitat-ui = { path = "crates/gitat-ui" }
thiserror = "2"
anyhow = "1"
ratatui = "0.30.0"
crossterm = "0.28"
similar = "2"
insta = "1.47.2"
```

- [ ] **Step 2: Create gitat-core crate**

`crates/gitat-core/Cargo.toml`:

```toml
[package]
name = "gitat-core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror = { workspace = true }

[dev-dependencies]
insta = { workspace = true }
```

`crates/gitat-core/src/lib.rs`:

```rust
pub mod runner;
```

- [ ] **Step 3: Create gitat-ui crate**

`crates/gitat-ui/Cargo.toml`:

```toml
[package]
name = "gitat-ui"
version.workspace = true
edition.workspace = true

[dependencies]
gitat-core = { workspace = true }
ratatui = { workspace = true }
crossterm = { workspace = true }

[dev-dependencies]
insta = { workspace = true }
```

`crates/gitat-ui/src/lib.rs`:

```rust
pub mod app;
```

- [ ] **Step 4: Create gitat binary crate**

`crates/gitat/Cargo.toml`:

```toml
[package]
name = "gitat"
version.workspace = true
edition.workspace = true

[dependencies]
gitat-core = { workspace = true }
gitat-ui = { workspace = true }
anyhow = { workspace = true }
crossterm = { workspace = true }
ratatui = { workspace = true }
```

`crates/gitat/src/main.rs`:

```rust
fn main() {
    println!("gitat - Git TUI Client");
}
```

- [ ] **Step 5: Create minimal app module for gitat-ui**

`crates/gitat-ui/src/app.rs`:

```rust
pub struct App {
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self { should_quit: false }
    }
}
```

- [ ] **Step 6: Create minimal runner module for gitat-core**

`crates/gitat-core/src/runner.rs`:

```rust
use std::path::PathBuf;

use crate::GitError;

pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
}

pub struct ProcessRunner {
    repo_path: PathBuf,
}

impl ProcessRunner {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }
}
```

Update `crates/gitat-core/src/lib.rs`:

```rust
pub mod runner;

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum GitError {
    #[error("git command failed: {command} (exit code {exit_code})\n{stderr}")]
    CommandFailed {
        command: String,
        stderr: String,
        exit_code: i32,
    },
    #[error("failed to parse git output: {0}")]
    ParseError(String),
    #[error("not a git repository")]
    NotARepository,
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::IoError(e.to_string())
    }
}
```

- [ ] **Step 7: Delete old src/main.rs and verify build**

```bash
rm src/main.rs && rmdir src
cargo build
```

Expected: Build succeeds with all 3 crates.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: convert to workspace with gitat-core, gitat-ui, gitat crates"
```

---

## Task 2: gitat-core — CommandRunner & ProcessRunner

Implement the git command execution layer.

**Files:**
- Modify: `crates/gitat-core/src/runner.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write failing test for ProcessRunner**

Add to `crates/gitat-core/src/runner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_runner_executes_git_version() {
        let runner = ProcessRunner::new(PathBuf::from("."));
        let result = runner.run(&["version"]);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("git version"));
    }

    #[test]
    fn test_process_runner_returns_error_for_invalid_command() {
        let runner = ProcessRunner::new(PathBuf::from("."));
        let result = runner.run(&["not-a-real-command"]);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p gitat-core
```

Expected: FAIL — `CommandRunner` trait not implemented for `ProcessRunner`.

- [ ] **Step 3: Implement ProcessRunner**

```rust
impl CommandRunner for ProcessRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
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
}
```

- [ ] **Step 4: Implement MockRunner for testing**

```rust
pub struct MockRunner {
    responses: std::collections::HashMap<String, Result<String, GitError>>,
}

impl MockRunner {
    pub fn new() -> Self {
        Self {
            responses: std::collections::HashMap::new(),
        }
    }

    pub fn with_response(mut self, key: &str, output: &str) -> Self {
        self.responses.insert(key.to_string(), Ok(output.to_string()));
        self
    }
}

impl CommandRunner for MockRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let key = args.join(" ");
        self.responses
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Err(GitError::CommandFailed {
                command: format!("git {}", key),
                stderr: "mock: no response configured".to_string(),
                exit_code: 1,
            }))
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p gitat-core
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-core/
git commit -m "feat(core): implement CommandRunner trait with ProcessRunner and MockRunner"
```

---

## Task 3: gitat-core — Status Module

Parse `git status --porcelain=v1` output into structured types.

**Files:**
- Create: `crates/gitat-core/src/status.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for status parsing**

`crates/gitat-core/src/status.rs`:

```rust
use crate::GitError;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Unmodified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusEntry {
    pub path: String,
    pub index_status: FileStatus,
    pub worktree_status: FileStatus,
}

pub fn parse_status(output: &str) -> Result<Vec<StatusEntry>, GitError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_status() {
        let result = parse_status("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_modified_file() {
        let result = parse_status(" M src/main.rs\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[0].index_status, FileStatus::Unmodified);
        assert_eq!(result[0].worktree_status, FileStatus::Modified);
    }

    #[test]
    fn test_parse_staged_and_modified() {
        let result = parse_status("MM src/lib.rs\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Modified);
        assert_eq!(result[0].worktree_status, FileStatus::Modified);
    }

    #[test]
    fn test_parse_added_file() {
        let result = parse_status("A  new_file.rs\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Added);
        assert_eq!(result[0].worktree_status, FileStatus::Unmodified);
    }

    #[test]
    fn test_parse_untracked() {
        let result = parse_status("?? untracked.txt\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Untracked);
        assert_eq!(result[0].worktree_status, FileStatus::Untracked);
    }

    #[test]
    fn test_parse_multiple_entries() {
        let input = " M src/main.rs\nA  new.rs\n?? notes.txt\n";
        let result = parse_status(input).unwrap();
        assert_eq!(result.len(), 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p gitat-core status
```

Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement parse_status**

```rust
fn parse_file_status(c: char) -> FileStatus {
    match c {
        'M' => FileStatus::Modified,
        'A' => FileStatus::Added,
        'D' => FileStatus::Deleted,
        'R' => FileStatus::Renamed,
        'C' => FileStatus::Copied,
        '?' => FileStatus::Untracked,
        ' ' => FileStatus::Unmodified,
        _ => FileStatus::Unmodified,
    }
}

pub fn parse_status(output: &str) -> Result<Vec<StatusEntry>, GitError> {
    let mut entries = Vec::new();

    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let index_status = parse_file_status(chars[0]);
        let worktree_status = parse_file_status(chars[1]);
        let path = line[3..].to_string();

        entries.push(StatusEntry {
            path,
            index_status,
            worktree_status,
        });
    }

    Ok(entries)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p gitat-core status
```

Expected: All tests PASS.

- [ ] **Step 5: Add get_status function using CommandRunner**

```rust
use crate::runner::CommandRunner;

pub fn get_status(runner: &dyn CommandRunner) -> Result<Vec<StatusEntry>, GitError> {
    let output = runner.run(&["status", "--porcelain=v1"])?;
    parse_status(&output)
}
```

- [ ] **Step 6: Update lib.rs and commit**

Add `pub mod status;` to `lib.rs`.

```bash
git add crates/gitat-core/
git commit -m "feat(core): add status module with porcelain parser"
```

---

## Task 4: gitat-core — Log Module

Parse `git log` output into structured types.

**Files:**
- Create: `crates/gitat-core/src/log.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for log parsing**

`crates/gitat-core/src/log.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub refs: Vec<String>,
    pub parent_hashes: Vec<String>,
}

/// Format string for git log: hash, short hash, parents, refs, author, date, subject
/// Fields separated by \x1f (unit separator), records separated by \x1e (record separator)
const LOG_FORMAT: &str = "%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e";

pub fn parse_log(output: &str) -> Result<Vec<CommitInfo>, GitError> {
    todo!()
}

pub fn get_log(runner: &dyn CommandRunner, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let output = runner.run(&[
        "log",
        &format!("--max-count={limit}"),
        &format!("--format={LOG_FORMAT}"),
    ])?;
    parse_log(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_log() {
        let result = parse_log("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_commit() {
        let input = "abc123def456\x1fabc123d\x1f\x1fHEAD -> main\x1fJohn Doe\x1f2026-04-03 10:00:00 +0900\x1ffeat: initial commit\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash, "abc123def456");
        assert_eq!(result[0].short_hash, "abc123d");
        assert_eq!(result[0].author, "John Doe");
        assert_eq!(result[0].message, "feat: initial commit");
        assert_eq!(result[0].refs, vec!["HEAD -> main"]);
        assert!(result[0].parent_hashes.is_empty());
    }

    #[test]
    fn test_parse_commit_with_parents() {
        let input = "abc123\x1fabc\x1fdef456 ghi789\x1f\x1fAlice\x1f2026-04-03\x1fmerge\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result[0].parent_hashes, vec!["def456", "ghi789"]);
    }

    #[test]
    fn test_parse_multiple_commits() {
        let input = "aaa\x1fa\x1f\x1fHEAD\x1fBob\x1f2026-04-03\x1ffirst\x1ebbb\x1fb\x1faaa\x1f\x1fBob\x1f2026-04-02\x1fsecond\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].message, "first");
        assert_eq!(result[1].message, "second");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p gitat-core log
```

Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement parse_log**

```rust
pub fn parse_log(output: &str) -> Result<Vec<CommitInfo>, GitError> {
    let mut commits = Vec::new();

    for record in output.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }

        let fields: Vec<&str> = record.split('\x1f').collect();
        if fields.len() < 7 {
            return Err(GitError::ParseError(format!(
                "expected 7 fields, got {}: {record}",
                fields.len()
            )));
        }

        let parent_hashes = if fields[2].is_empty() {
            Vec::new()
        } else {
            fields[2].split(' ').map(|s| s.to_string()).collect()
        };

        let refs = if fields[3].is_empty() {
            Vec::new()
        } else {
            fields[3].split(", ").map(|s| s.to_string()).collect()
        };

        commits.push(CommitInfo {
            hash: fields[0].to_string(),
            short_hash: fields[1].to_string(),
            parent_hashes,
            refs,
            author: fields[4].to_string(),
            date: fields[5].to_string(),
            message: fields[6].to_string(),
        });
    }

    Ok(commits)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p gitat-core log
```

Expected: All tests PASS.

- [ ] **Step 5: Update lib.rs and commit**

Add `pub mod log;` to `lib.rs`.

```bash
git add crates/gitat-core/
git commit -m "feat(core): add log module with structured commit parsing"
```

---

## Task 5: gitat-core — Diff Module

Parse unified diff output into structured types.

**Files:**
- Create: `crates/gitat-core/src/diff.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write types and failing tests**

`crates/gitat-core/src/diff.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffFile {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

pub fn parse_diff(output: &str) -> Result<Vec<DiffFile>, GitError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_diff() {
        let result = parse_diff("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_file_diff() {
        let input = "\
diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    println!(\"hello\");
+    let msg = \"hello\";
+    println!(\"{msg}\");
 }
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "src/main.rs");
        assert_eq!(result[0].new_path, "src/main.rs");
        assert_eq!(result[0].hunks.len(), 1);

        let hunk = &result[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.lines.len(), 5);
    }

    #[test]
    fn test_diff_line_numbers() {
        let input = "\
diff --git a/f.rs b/f.rs
--- a/f.rs
+++ b/f.rs
@@ -10,3 +10,4 @@
 context
-removed
+added1
+added2
 context2
";
        let result = parse_diff(input).unwrap();
        let lines = &result[0].hunks[0].lines;

        assert_eq!(lines[0].old_line_no, Some(10));
        assert_eq!(lines[0].new_line_no, Some(10));
        assert_eq!(lines[1].old_line_no, Some(11));
        assert_eq!(lines[1].new_line_no, None);
        assert_eq!(lines[2].old_line_no, None);
        assert_eq!(lines[2].new_line_no, Some(11));
        assert_eq!(lines[3].old_line_no, None);
        assert_eq!(lines[3].new_line_no, Some(12));
        assert_eq!(lines[4].old_line_no, Some(12));
        assert_eq!(lines[4].new_line_no, Some(13));
    }

    #[test]
    fn test_parse_new_file() {
        let input = "\
diff --git a/new.rs b/new.rs
new file mode 100644
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,2 @@
+fn hello() {}
+fn world() {}
";
        let result = parse_diff(input).unwrap();
        assert_eq!(result[0].old_path, "/dev/null");
        assert_eq!(result[0].new_path, "new.rs");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p gitat-core diff
```

Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement parse_diff**

```rust
pub fn parse_diff(output: &str) -> Result<Vec<DiffFile>, GitError> {
    let mut files = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    for line in output.lines() {
        if line.starts_with("diff --git") {
            // Flush previous hunk/file
            if let Some(ref mut file) = current_file {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                files.push(file.clone());
            }
            current_file = Some(DiffFile {
                old_path: String::new(),
                new_path: String::new(),
                hunks: Vec::new(),
            });
            current_hunk = None;
        } else if line.starts_with("--- ") {
            if let Some(ref mut file) = current_file {
                let path = line.strip_prefix("--- ").unwrap_or("");
                file.old_path = path.strip_prefix("a/").unwrap_or(path).to_string();
            }
        } else if line.starts_with("+++ ") {
            if let Some(ref mut file) = current_file {
                let path = line.strip_prefix("+++ ").unwrap_or("");
                file.new_path = path.strip_prefix("b/").unwrap_or(path).to_string();
            }
        } else if line.starts_with("@@ ") {
            // Flush previous hunk
            if let Some(ref mut file) = current_file {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
            }
            // Parse @@ -old_start,old_count +new_start,new_count @@
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let old = parts[1].strip_prefix('-').unwrap_or(parts[1]);
                let new = parts[2].strip_prefix('+').unwrap_or(parts[2]);
                let (os, oc) = parse_range(old);
                let (ns, nc) = parse_range(new);
                old_line = os;
                new_line = ns;
                current_hunk = Some(DiffHunk {
                    old_start: os,
                    old_count: oc,
                    new_start: ns,
                    new_count: nc,
                    lines: Vec::new(),
                });
            }
        } else if let Some(ref mut hunk) = current_hunk {
            if let Some(content) = line.strip_prefix('+') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    content: content.to_string(),
                    old_line_no: None,
                    new_line_no: Some(new_line),
                });
                new_line += 1;
            } else if let Some(content) = line.strip_prefix('-') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Removed,
                    content: content.to_string(),
                    old_line_no: Some(old_line),
                    new_line_no: None,
                });
                old_line += 1;
            } else if let Some(content) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    content: content.to_string(),
                    old_line_no: Some(old_line),
                    new_line_no: Some(new_line),
                });
                old_line += 1;
                new_line += 1;
            }
        }
    }

    // Flush last hunk/file
    if let Some(ref mut file) = current_file {
        if let Some(hunk) = current_hunk.take() {
            file.hunks.push(hunk);
        }
        files.push(file.clone());
    }

    Ok(files)
}

fn parse_range(s: &str) -> (u32, u32) {
    if let Some((start, count)) = s.split_once(',') {
        (
            start.parse().unwrap_or(0),
            count.parse().unwrap_or(0),
        )
    } else {
        (s.parse().unwrap_or(0), 1)
    }
}
```

- [ ] **Step 4: Add get_diff functions**

```rust
pub fn get_diff(runner: &dyn CommandRunner, staged: bool) -> Result<Vec<DiffFile>, GitError> {
    let output = if staged {
        runner.run(&["diff", "--cached"])?
    } else {
        runner.run(&["diff"])?
    };
    parse_diff(&output)
}

pub fn get_diff_for_file(
    runner: &dyn CommandRunner,
    path: &str,
    staged: bool,
) -> Result<Vec<DiffFile>, GitError> {
    let output = if staged {
        runner.run(&["diff", "--cached", "--", path])?
    } else {
        runner.run(&["diff", "--", path])?
    };
    parse_diff(&output)
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p gitat-core diff
```

Expected: All tests PASS.

- [ ] **Step 6: Update lib.rs and commit**

Add `pub mod diff;` to `lib.rs`.

```bash
git add crates/gitat-core/
git commit -m "feat(core): add diff module with unified diff parser"
```

---

## Task 6: gitat-core — Branch, Stage, Commit, Remote Modules

Implement remaining operation modules. These are thinner wrappers around git commands.

**Files:**
- Create: `crates/gitat-core/src/branch.rs`
- Create: `crates/gitat-core/src/stage.rs`
- Create: `crates/gitat-core/src/commit.rs`
- Create: `crates/gitat-core/src/remote.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write branch module with tests**

`crates/gitat-core/src/branch.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub last_commit: String,
}

pub fn parse_branches(output: &str) -> Result<Vec<BranchInfo>, GitError> {
    let mut branches = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_current = line.starts_with('*');
        let line = line.trim_start_matches(['*', ' ']);

        // Format: "name hash commit message" or "name -> origin/main"
        if line.contains(" -> ") {
            continue; // Skip symbolic refs like HEAD -> origin/main
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            branches.push(BranchInfo {
                name: parts[0].to_string(),
                is_current,
                upstream: None,
                last_commit: parts.get(2).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(branches)
}

pub fn list_branches(runner: &dyn CommandRunner) -> Result<Vec<BranchInfo>, GitError> {
    let output = runner.run(&["branch", "-v", "--no-color"])?;
    parse_branches(&output)
}

pub fn create_branch(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["branch", name])?;
    Ok(())
}

pub fn checkout(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["checkout", name])?;
    Ok(())
}

pub fn delete_branch(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["branch", "-d", name])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branches() {
        let input = "* main       abc1234 latest commit\n  feature    def5678 wip feature\n";
        let result = parse_branches(input).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_current);
        assert_eq!(result[0].name, "main");
        assert!(!result[1].is_current);
        assert_eq!(result[1].name, "feature");
    }

    #[test]
    fn test_parse_empty_branches() {
        let result = parse_branches("").unwrap();
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 2: Write stage module**

`crates/gitat-core/src/stage.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

pub fn stage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["add", "--", path])?;
    Ok(())
}

pub fn unstage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["reset", "HEAD", "--", path])?;
    Ok(())
}
```

- [ ] **Step 3: Write commit module**

`crates/gitat-core/src/commit.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

pub fn commit(runner: &dyn CommandRunner, message: &str) -> Result<(), GitError> {
    runner.run(&["commit", "-m", message])?;
    Ok(())
}

pub fn amend(runner: &dyn CommandRunner, message: &str) -> Result<(), GitError> {
    runner.run(&["commit", "--amend", "-m", message])?;
    Ok(())
}
```

- [ ] **Step 4: Write remote module**

`crates/gitat-core/src/remote.rs`:

```rust
use crate::GitError;
use crate::runner::CommandRunner;

pub fn push(runner: &dyn CommandRunner, remote: &str, branch: &str) -> Result<(), GitError> {
    runner.run(&["push", remote, branch])?;
    Ok(())
}

pub fn pull(runner: &dyn CommandRunner, remote: &str, branch: &str) -> Result<(), GitError> {
    runner.run(&["pull", remote, branch])?;
    Ok(())
}

pub fn fetch(runner: &dyn CommandRunner) -> Result<(), GitError> {
    runner.run(&["fetch", "--all"])?;
    Ok(())
}
```

- [ ] **Step 5: Update lib.rs with all modules**

```rust
pub mod runner;
pub mod status;
pub mod log;
pub mod diff;
pub mod branch;
pub mod stage;
pub mod commit;
pub mod remote;

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum GitError {
    #[error("git command failed: {command} (exit code {exit_code})\n{stderr}")]
    CommandFailed {
        command: String,
        stderr: String,
        exit_code: i32,
    },
    #[error("failed to parse git output: {0}")]
    ParseError(String),
    #[error("not a git repository")]
    NotARepository,
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::IoError(e.to_string())
    }
}
```

Note: This `lib.rs` now includes all modules. The `GitError` definition is unchanged from Task 1 (already uses `String` for `IoError` and derives `Clone`).

- [ ] **Step 6: Run all core tests**

```bash
cargo test -p gitat-core
```

Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/gitat-core/
git commit -m "feat(core): add branch, stage, commit, and remote modules"
```

---

## Task 7: gitat-core — Conflict Module

Parse conflict markers and support resolution.

**Files:**
- Create: `crates/gitat-core/src/conflict.rs`
- Modify: `crates/gitat-core/src/lib.rs`

- [ ] **Step 1: Write types and failing tests**

`crates/gitat-core/src/conflict.rs`:

```rust
use std::path::Path;

use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictRegion {
    Clean(Vec<String>),
    Conflict {
        ours: Vec<String>,
        theirs: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictFile {
    pub path: String,
    pub regions: Vec<ConflictRegion>,
}

pub fn parse_conflict_markers(path: &str, content: &str) -> Result<ConflictFile, GitError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflicts() {
        let content = "line1\nline2\nline3\n";
        let result = parse_conflict_markers("file.rs", content).unwrap();
        assert_eq!(result.regions.len(), 1);
        match &result.regions[0] {
            ConflictRegion::Clean(lines) => assert_eq!(lines.len(), 3),
            _ => panic!("expected Clean region"),
        }
    }

    #[test]
    fn test_single_conflict() {
        let content = "\
line1
<<<<<<< HEAD
ours line
=======
theirs line
>>>>>>> feature
line2
";
        let result = parse_conflict_markers("file.rs", content).unwrap();
        assert_eq!(result.regions.len(), 3);
        match &result.regions[0] {
            ConflictRegion::Clean(lines) => assert_eq!(lines, &["line1"]),
            _ => panic!("expected Clean"),
        }
        match &result.regions[1] {
            ConflictRegion::Conflict { ours, theirs } => {
                assert_eq!(ours, &["ours line"]);
                assert_eq!(theirs, &["theirs line"]);
            }
            _ => panic!("expected Conflict"),
        }
        match &result.regions[2] {
            ConflictRegion::Clean(lines) => assert_eq!(lines, &["line2"]),
            _ => panic!("expected Clean"),
        }
    }

    #[test]
    fn test_multiple_conflicts() {
        let content = "\
<<<<<<< HEAD
a
=======
b
>>>>>>> feat
middle
<<<<<<< HEAD
c
=======
d
>>>>>>> feat
";
        let result = parse_conflict_markers("f.rs", content).unwrap();
        assert_eq!(result.regions.len(), 3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p gitat-core conflict
```

Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement parse_conflict_markers**

```rust
pub fn parse_conflict_markers(path: &str, content: &str) -> Result<ConflictFile, GitError> {
    let mut regions = Vec::new();
    let mut clean_lines = Vec::new();
    let mut ours_lines: Option<Vec<String>> = None;
    let mut theirs_lines: Option<Vec<String>> = None;
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            if !clean_lines.is_empty() {
                regions.push(ConflictRegion::Clean(std::mem::take(&mut clean_lines)));
            }
            in_ours = true;
            ours_lines = Some(Vec::new());
        } else if line.starts_with("=======") && in_ours {
            in_ours = false;
            in_theirs = true;
            theirs_lines = Some(Vec::new());
        } else if line.starts_with(">>>>>>>") && in_theirs {
            in_theirs = false;
            if let (Some(ours), Some(theirs)) = (ours_lines.take(), theirs_lines.take()) {
                regions.push(ConflictRegion::Conflict { ours, theirs });
            }
        } else if in_ours {
            if let Some(ref mut lines) = ours_lines {
                lines.push(line.to_string());
            }
        } else if in_theirs {
            if let Some(ref mut lines) = theirs_lines {
                lines.push(line.to_string());
            }
        } else {
            clean_lines.push(line.to_string());
        }
    }

    if !clean_lines.is_empty() {
        regions.push(ConflictRegion::Clean(clean_lines));
    }

    Ok(ConflictFile {
        path: path.to_string(),
        regions,
    })
}
```

- [ ] **Step 4: Add get_conflicts and resolve_file**

```rust
pub fn get_conflict_paths(runner: &dyn CommandRunner) -> Result<Vec<String>, GitError> {
    let output = runner.run(&["diff", "--name-only", "--diff-filter=U"])?;
    Ok(output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
}

pub fn resolve_file(runner: &dyn CommandRunner, path: &str, content: &str) -> Result<(), GitError> {
    std::fs::write(Path::new(path), content)
        .map_err(|e| GitError::IoError(e.to_string()))?;
    runner.run(&["add", "--", path])?;
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p gitat-core conflict
```

Expected: All tests PASS.

- [ ] **Step 6: Update lib.rs and commit**

Add `pub mod conflict;` to `lib.rs`.

```bash
git add crates/gitat-core/
git commit -m "feat(core): add conflict module with marker parser and resolution"
```

---

## Task 8: gitat-ui — App State, Theme, Event Handling

Set up the core UI infrastructure.

**Files:**
- Modify: `crates/gitat-ui/src/app.rs`
- Create: `crates/gitat-ui/src/theme.rs`
- Create: `crates/gitat-ui/src/event.rs`
- Modify: `crates/gitat-ui/src/lib.rs`

- [ ] **Step 1: Write App state with tests**

`crates/gitat-ui/src/app.rs`:

```rust
use gitat_core::branch::BranchInfo;
use gitat_core::diff::DiffFile;
use gitat_core::log::CommitInfo;
use gitat_core::runner::CommandRunner;
use gitat_core::status::StatusEntry;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Status,
    Branches,
    Log,
    Stash,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Status => Tab::Branches,
            Tab::Branches => Tab::Log,
            Tab::Log => Tab::Stash,
            Tab::Stash => Tab::Status,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Status => Tab::Stash,
            Tab::Branches => Tab::Status,
            Tab::Log => Tab::Branches,
            Tab::Stash => Tab::Log,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Status => "Status",
            Tab::Branches => "Branches",
            Tab::Log => "Log",
            Tab::Stash => "Stash",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Commit { message: String },
    Conflict { file: String },
    Search { query: String },
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Left,
    Right,
}

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub panel: Panel,
    pub should_quit: bool,

    pub file_list_state: ListState,
    pub diff_scroll: (u16, u16),

    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log_entries: Vec<CommitInfo>,
    pub current_diff: Option<Vec<DiffFile>>,

    pub status_message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: Tab::Status,
            mode: Mode::Normal,
            panel: Panel::Left,
            should_quit: false,
            file_list_state: ListState::default(),
            diff_scroll: (0, 0),
            status: Vec::new(),
            branches: Vec::new(),
            log_entries: Vec::new(),
            current_diff: None,
            status_message: None,
        }
    }

    pub fn refresh(&mut self, runner: &dyn CommandRunner) {
        if let Ok(status) = gitat_core::status::get_status(runner) {
            self.status = status;
        }
        if let Ok(branches) = gitat_core::branch::list_branches(runner) {
            self.branches = branches;
        }
        if let Ok(log) = gitat_core::log::get_log(runner, 100) {
            self.log_entries = log;
        }
    }

    pub fn set_status_message(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_cycling() {
        assert_eq!(Tab::Status.next(), Tab::Branches);
        assert_eq!(Tab::Stash.next(), Tab::Status);
        assert_eq!(Tab::Status.prev(), Tab::Stash);
        assert_eq!(Tab::Branches.prev(), Tab::Status);
    }

    #[test]
    fn test_app_initial_state() {
        let app = App::new();
        assert_eq!(app.tab, Tab::Status);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.panel, Panel::Left);
        assert!(!app.should_quit);
    }
}
```

- [ ] **Step 2: Write theme module**

`crates/gitat-ui/src/theme.rs`:

```rust
use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn tab_active() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub fn file_staged() -> Style {
        Style::new().fg(Color::Green)
    }

    pub fn file_unstaged() -> Style {
        Style::new().fg(Color::Red)
    }

    pub fn file_untracked() -> Style {
        Style::new().fg(Color::Gray)
    }

    pub fn diff_added() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(28, 61, 28))
    }

    pub fn diff_removed() -> Style {
        Style::new().fg(Color::Red).bg(Color::Rgb(61, 28, 28))
    }

    pub fn diff_word_added() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(44, 107, 44))
    }

    pub fn diff_word_removed() -> Style {
        Style::new().fg(Color::Red).bg(Color::Rgb(107, 44, 44))
    }

    pub fn diff_context() -> Style {
        Style::new().fg(Color::Gray)
    }

    pub fn diff_line_number() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub fn diff_hunk_header() -> Style {
        Style::new().fg(Color::Cyan)
    }

    pub fn status_bar() -> Style {
        Style::new().fg(Color::White).bg(Color::DarkGray)
    }

    pub fn help_key() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub fn conflict_ours() -> Style {
        Style::new().fg(Color::Rgb(232, 168, 56)).bg(Color::Rgb(42, 34, 16))
    }

    pub fn conflict_theirs() -> Style {
        Style::new().fg(Color::Rgb(56, 168, 232)).bg(Color::Rgb(16, 34, 42))
    }

    pub fn conflict_result() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(13, 26, 13))
    }

    pub fn selected() -> Style {
        Style::new().bg(Color::Rgb(50, 50, 80)).add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub fn border_focused() -> Style {
        Style::new().fg(Color::Cyan)
    }

    pub fn branch_current() -> Style {
        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
    }

    pub fn commit_hash() -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn commit_ref() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }
}
```

- [ ] **Step 3: Write event handling module**

`crates/gitat-ui/src/event.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gitat_core::runner::CommandRunner;

use crate::app::{App, Mode, Panel, Tab};

pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match &app.mode {
        Mode::Normal => handle_normal_mode(app, key, runner),
        Mode::Commit { .. } => handle_commit_mode(app, key, runner),
        Mode::Help => handle_help_mode(app, key),
        Mode::Search { .. } => handle_search_mode(app, key),
        Mode::Conflict { .. } => {} // Handled by conflict editor
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') => move_down(app),
        KeyCode::Char('k') => move_up(app),
        KeyCode::Char('h') => app.panel = Panel::Left,
        KeyCode::Char('l') => app.panel = Panel::Right,
        KeyCode::Tab => app.tab = app.tab.next(),
        KeyCode::BackTab => app.tab = app.tab.prev(),
        KeyCode::Char('s') if app.tab == Tab::Status => toggle_stage(app, runner),
        KeyCode::Char('c') => {
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('p') => {
            if let Err(e) = gitat_core::remote::push(runner, "origin", "HEAD") {
                app.set_status_message(format!("Push failed: {e}"));
            } else {
                app.set_status_message("Pushed successfully");
            }
        }
        KeyCode::Char('P') => {
            if let Err(e) = gitat_core::remote::pull(runner, "origin", "HEAD") {
                app.set_status_message(format!("Pull failed: {e}"));
            } else {
                app.set_status_message("Pulled successfully");
                app.refresh(runner);
            }
        }
        KeyCode::Char('b') if app.tab == Tab::Branches => {
            // TODO: prompt for branch name via Mode::Input
            app.set_status_message("Branch creation: not yet implemented");
        }
        KeyCode::Char('d') if app.tab == Tab::Branches => {
            if let Some(idx) = app.file_list_state.selected() {
                if let Some(branch) = app.branches.get(idx) {
                    let name = branch.name.clone();
                    if let Err(e) = gitat_core::branch::delete_branch(runner, &name) {
                        app.set_status_message(format!("Delete failed: {e}"));
                    } else {
                        app.set_status_message(format!("Deleted branch: {name}"));
                        app.refresh(runner);
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search { query: String::new() };
        }
        KeyCode::Char('r') => app.refresh(runner),
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Enter => load_diff_for_selected(app, runner),
        _ => {}
    }
}

fn handle_commit_mode(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    if let Mode::Commit { ref mut message } = app.mode {
        match key.code {
            KeyCode::Esc => app.mode = Mode::Normal,
            KeyCode::Enter => {
                let msg = message.clone();
                if !msg.is_empty() {
                    if let Err(e) = gitat_core::commit::commit(runner, &msg) {
                        app.set_status_message(format!("Commit failed: {e}"));
                    } else {
                        app.set_status_message("Committed successfully");
                        app.refresh(runner);
                    }
                }
                app.mode = Mode::Normal;
            }
            KeyCode::Char(c) => message.push(c),
            KeyCode::Backspace => { message.pop(); }
            _ => {}
        }
    }
}

fn handle_help_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    if let Mode::Search { ref mut query } = app.mode {
        match key.code {
            KeyCode::Esc => app.mode = Mode::Normal,
            KeyCode::Char(c) => query.push(c),
            KeyCode::Backspace => { query.pop(); }
            _ => {}
        }
    }
}

fn move_down(app: &mut App) {
    let len = match app.tab {
        Tab::Status => app.status.len(),
        Tab::Branches => app.branches.len(),
        Tab::Log => app.log_entries.len(),
        Tab::Stash => 0,
    };
    if len == 0 {
        return;
    }
    let i = app.file_list_state.selected().map_or(0, |i| {
        if i + 1 < len { i + 1 } else { i }
    });
    app.file_list_state.select(Some(i));
}

fn move_up(app: &mut App) {
    let i = app.file_list_state.selected().map_or(0, |i| i.saturating_sub(1));
    app.file_list_state.select(Some(i));
}

fn toggle_stage(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(idx) = app.file_list_state.selected() {
        if let Some(entry) = app.status.get(idx) {
            let path = entry.path.clone();
            let result = if entry.index_status != gitat_core::status::FileStatus::Unmodified
                && entry.index_status != gitat_core::status::FileStatus::Untracked
            {
                gitat_core::stage::unstage_file(runner, &path)
            } else {
                gitat_core::stage::stage_file(runner, &path)
            };
            if let Err(e) = result {
                app.set_status_message(format!("Stage/unstage failed: {e}"));
            }
            app.refresh(runner);
        }
    }
}

fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if app.tab != Tab::Status {
        return;
    }
    if let Some(idx) = app.file_list_state.selected() {
        if let Some(entry) = app.status.get(idx) {
            let staged = entry.index_status != gitat_core::status::FileStatus::Unmodified
                && entry.index_status != gitat_core::status::FileStatus::Untracked;
            match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
                Ok(diff) => {
                    app.current_diff = Some(diff);
                    app.diff_scroll = (0, 0);
                    app.panel = Panel::Right;
                }
                Err(e) => app.set_status_message(format!("Diff failed: {e}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_quit() {
        let mut app = App::new();
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('q')), &runner);
        assert!(app.should_quit);
    }

    #[test]
    fn test_tab_switch() {
        let mut app = App::new();
        let runner = MockRunner::new();
        assert_eq!(app.tab, Tab::Status);
        handle_key(&mut app, mock_key(KeyCode::Tab), &runner);
        assert_eq!(app.tab, Tab::Branches);
    }

    #[test]
    fn test_panel_switch() {
        let mut app = App::new();
        let runner = MockRunner::new();
        assert_eq!(app.panel, Panel::Left);
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn test_enter_commit_mode() {
        let mut app = App::new();
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
    }

    #[test]
    fn test_escape_commit_mode() {
        let mut app = App::new();
        app.mode = Mode::Commit { message: String::new() };
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
pub mod app;
pub mod event;
pub mod theme;
pub mod views;
pub mod widgets;
```

Create stub modules:

`crates/gitat-ui/src/views/mod.rs`:
```rust
pub mod status;
pub mod log;
pub mod branches;
pub mod commit;
```

`crates/gitat-ui/src/widgets/mod.rs`:
```rust
pub mod side_by_side_diff;
pub mod conflict_editor;
```

Create empty stub files for each view and widget module (just `// TODO` for now).

- [ ] **Step 5: Run tests**

```bash
cargo test -p gitat-ui
```

Expected: All tests PASS. (View/widget stubs compile but have no tests yet.)

- [ ] **Step 6: Commit**

```bash
git add crates/gitat-ui/
git commit -m "feat(ui): add App state, theme, event handling, and module stubs"
```

---

## Task 9: gitat-ui — Status View

Implement the Status tab view with file list and diff detail panel.

**Files:**
- Modify: `crates/gitat-ui/src/views/status.rs`
- Modify: `crates/gitat-ui/src/views/mod.rs`

- [ ] **Step 1: Implement status view rendering**

`crates/gitat-ui/src/views/status.rs`:

```rust
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};

use crate::app::{App, Panel, Tab};
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    render_file_list(f, app, chunks[0]);
    render_detail(f, app, chunks[1]);
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.panel == Panel::Left {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let mut items: Vec<ListItem> = Vec::new();

    // Staged files
    let staged: Vec<&gitat_core::status::StatusEntry> = app
        .status
        .iter()
        .filter(|e| {
            e.index_status != gitat_core::status::FileStatus::Unmodified
                && e.index_status != gitat_core::status::FileStatus::Untracked
        })
        .collect();

    if !staged.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("Staged ({})", staged.len()),
            Style::new().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD),
        ))));
        for entry in &staged {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  {:?} {}", entry.index_status, entry.path),
                Theme::file_staged(),
            ))));
        }
    }

    // Unstaged files
    let unstaged: Vec<&gitat_core::status::StatusEntry> = app
        .status
        .iter()
        .filter(|e| {
            e.worktree_status != gitat_core::status::FileStatus::Unmodified
                && e.index_status != gitat_core::status::FileStatus::Untracked
        })
        .collect();

    if !unstaged.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("Modified ({})", unstaged.len()),
            Style::new().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD),
        ))));
        for entry in &unstaged {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  {:?} {}", entry.worktree_status, entry.path),
                Theme::file_unstaged(),
            ))));
        }
    }

    // Untracked files
    let untracked: Vec<&gitat_core::status::StatusEntry> = app
        .status
        .iter()
        .filter(|e| e.index_status == gitat_core::status::FileStatus::Untracked)
        .collect();

    if !untracked.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("Untracked ({})", untracked.len()),
            Style::new().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD),
        ))));
        for entry in &untracked {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  ? {}", entry.path),
                Theme::file_untracked(),
            ))));
        }
    }

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.panel == Panel::Right {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.current_diff {
        // Render diff — delegate to side_by_side_diff widget (Task 10)
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let placeholder = Paragraph::new("Diff content will be rendered here")
            .block(block);
        f.render_widget(placeholder, area);
    } else {
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let placeholder = Paragraph::new("Select a file to view diff")
            .block(block);
        f.render_widget(placeholder, area);
    }
}
```

- [ ] **Step 2: Write render dispatch in views/mod.rs**

```rust
pub mod status;
pub mod log;
pub mod branches;
pub mod commit;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::{App, Tab};

pub fn render_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Status => status::render(f, app, area),
        Tab::Log => log::render(f, app, area),
        Tab::Branches => branches::render(f, app, area),
        Tab::Stash => {} // Placeholder
    }
}
```

- [ ] **Step 3: Write placeholder log and branches views**

`crates/gitat-ui/src/views/log.rs`:

```rust
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .log_entries
        .iter()
        .map(|c| {
            let mut spans = vec![
                Span::styled(&c.short_hash, Theme::commit_hash()),
                Span::raw(" "),
            ];
            if !c.refs.is_empty() {
                spans.push(Span::styled(
                    format!("({}) ", c.refs.join(", ")),
                    Theme::commit_ref(),
                ));
            }
            spans.push(Span::raw(&c.message));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}
```

`crates/gitat-ui/src/views/branches.rs`:

```rust
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .branches
        .iter()
        .map(|b| {
            let style = if b.is_current {
                Theme::branch_current()
            } else {
                Theme::diff_context()
            };
            let prefix = if b.is_current { "* " } else { "  " };
            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{}", b.name),
                style,
            )))
        })
        .collect();

    let block = Block::default()
        .title(" Branches ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}
```

`crates/gitat-ui/src/views/commit.rs`:

```rust
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, Mode};
use crate::theme::Theme;

pub fn render_commit_popup(f: &mut Frame, app: &App) {
    if let Mode::Commit { ref message } = app.mode {
        let area = centered_rect(60, 20, f.area());

        f.render_widget(Clear, area);

        let block = Block::default()
            .title(" Commit Message (Enter: commit, Esc: cancel) ")
            .borders(Borders::ALL)
            .border_style(Theme::border_focused());

        let paragraph = Paragraph::new(format!("{message}█"))
            .block(block);

        f.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
```

- [ ] **Step 4: Run tests and verify build**

```bash
cargo test -p gitat-ui && cargo build
```

Expected: All tests PASS, build succeeds.

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/
git commit -m "feat(ui): implement status, log, branches views and commit popup"
```

---

## Task 10: gitat-ui — Side-by-side Diff Widget

The primary differentiator. Custom `StatefulWidget` for side-by-side diff display.

**Files:**
- Modify: `crates/gitat-ui/src/widgets/side_by_side_diff.rs`
- Modify: `crates/gitat-ui/Cargo.toml` (add `similar` dependency)

- [ ] **Step 1: Add similar dependency**

Add to `crates/gitat-ui/Cargo.toml`:

```toml
[dependencies]
similar = { workspace = true }
```

- [ ] **Step 2: Write the widget state and types**

`crates/gitat-ui/src/widgets/side_by_side_diff.rs`:

```rust
use gitat_core::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use similar::{ChangeTag, TextDiff};

use crate::theme::Theme;

/// A paired row in the side-by-side display.
#[derive(Debug, Clone)]
struct DiffRow {
    left_line_no: Option<u32>,
    left_content: Option<String>,
    left_kind: DiffLineKind,
    right_line_no: Option<u32>,
    right_content: Option<String>,
    right_kind: DiffLineKind,
}

pub struct SideBySideDiffState {
    pub scroll_y: u16,
    pub scroll_x: u16,
    pub current_hunk: usize,
    hunk_offsets: Vec<u16>, // row offset of each hunk start
}

impl SideBySideDiffState {
    pub fn new() -> Self {
        Self {
            scroll_y: 0,
            scroll_x: 0,
            current_hunk: 0,
            hunk_offsets: Vec::new(),
        }
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_y = self.scroll_y.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_y = self.scroll_y.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: u16) {
        self.scroll_x = self.scroll_x.saturating_add(amount);
    }

    pub fn scroll_left(&mut self, amount: u16) {
        self.scroll_x = self.scroll_x.saturating_sub(amount);
    }

    pub fn next_hunk(&mut self) {
        if self.current_hunk + 1 < self.hunk_offsets.len() {
            self.current_hunk += 1;
            self.scroll_y = self.hunk_offsets[self.current_hunk];
        }
    }

    pub fn prev_hunk(&mut self) {
        if self.current_hunk > 0 {
            self.current_hunk -= 1;
            self.scroll_y = self.hunk_offsets[self.current_hunk];
        }
    }
}

pub struct SideBySideDiff<'a> {
    diff_files: &'a [DiffFile],
    block: Option<Block<'a>>,
}

impl<'a> SideBySideDiff<'a> {
    pub fn new(diff_files: &'a [DiffFile]) -> Self {
        Self {
            diff_files,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}
```

- [ ] **Step 3: Implement line pairing logic**

```rust
/// Pair old/new lines within a hunk for side-by-side display.
fn pair_lines(hunk: &DiffHunk) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let mut removed_buf: Vec<&DiffLine> = Vec::new();
    let mut added_buf: Vec<&DiffLine> = Vec::new();

    for line in &hunk.lines {
        match line.kind {
            DiffLineKind::Context => {
                flush_buffers(&mut rows, &mut removed_buf, &mut added_buf);
                rows.push(DiffRow {
                    left_line_no: line.old_line_no,
                    left_content: Some(line.content.clone()),
                    left_kind: DiffLineKind::Context,
                    right_line_no: line.new_line_no,
                    right_content: Some(line.content.clone()),
                    right_kind: DiffLineKind::Context,
                });
            }
            DiffLineKind::Removed => removed_buf.push(line),
            DiffLineKind::Added => added_buf.push(line),
        }
    }
    flush_buffers(&mut rows, &mut removed_buf, &mut added_buf);
    rows
}

fn flush_buffers(rows: &mut Vec<DiffRow>, removed: &mut Vec<&DiffLine>, added: &mut Vec<&DiffLine>) {
    let max_len = removed.len().max(added.len());
    for i in 0..max_len {
        let left = removed.get(i);
        let right = added.get(i);
        rows.push(DiffRow {
            left_line_no: left.and_then(|l| l.old_line_no),
            left_content: left.map(|l| l.content.clone()),
            left_kind: if left.is_some() { DiffLineKind::Removed } else { DiffLineKind::Context },
            right_line_no: right.and_then(|l| l.new_line_no),
            right_content: right.map(|l| l.content.clone()),
            right_kind: if right.is_some() { DiffLineKind::Added } else { DiffLineKind::Context },
        });
    }
    removed.clear();
    added.clear();
}
```

- [ ] **Step 4: Implement word-level diff rendering**

```rust
/// Render a single side's content with word-level highlighting.
fn render_content_with_word_diff(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    scroll_x: u16,
    left: Option<&str>,
    right: Option<&str>,
    is_left_side: bool,
    kind: &DiffLineKind,
) {
    let content = if is_left_side { left } else { right };
    let content = match content {
        Some(c) => c,
        None => {
            // Empty padding line
            let style = Style::new().bg(ratatui::style::Color::Rgb(30, 30, 30));
            for dx in 0..width {
                if x + dx < buf.area().right() && y < buf.area().bottom() {
                    buf.cell_mut((x + dx, y)).map(|cell| cell.set_style(style));
                }
            }
            return;
        }
    };

    let base_style = match kind {
        DiffLineKind::Added => Theme::diff_added(),
        DiffLineKind::Removed => Theme::diff_removed(),
        DiffLineKind::Context => Theme::diff_context(),
    };

    // If we have both sides and it's a change, do word-level diff
    if let (Some(old), Some(new)) = (left, right) {
        if *kind != DiffLineKind::Context {
            let text_diff = TextDiff::from_words(old, new);
            let source = if is_left_side { old } else { new };
            let highlight_tag = if is_left_side { ChangeTag::Delete } else { ChangeTag::Insert };
            let word_style = if is_left_side { Theme::diff_word_removed() } else { Theme::diff_word_added() };

            let mut col: u16 = 0;
            for change in text_diff.iter_all_changes() {
                let text = change.value();
                let style = if change.tag() == highlight_tag {
                    word_style
                } else if change.tag() == ChangeTag::Equal {
                    base_style
                } else {
                    continue; // Skip the other side's changes
                };

                for ch in text.chars() {
                    if col >= scroll_x && (col - scroll_x) < width {
                        let dx = col - scroll_x;
                        if x + dx < buf.area().right() && y < buf.area().bottom() {
                            buf.cell_mut((x + dx, y)).map(|cell| {
                                cell.set_char(ch);
                                cell.set_style(style);
                            });
                        }
                    }
                    col += 1;
                }
            }
            // Fill remaining with base style
            let start = col.saturating_sub(scroll_x);
            for dx in start..width {
                if x + dx < buf.area().right() && y < buf.area().bottom() {
                    buf.cell_mut((x + dx, y)).map(|cell| cell.set_style(base_style));
                }
            }
            return;
        }
    }

    // Simple rendering without word-level diff
    let mut col: u16 = 0;
    for ch in content.chars() {
        if col >= scroll_x && (col - scroll_x) < width {
            let dx = col - scroll_x;
            if x + dx < buf.area().right() && y < buf.area().bottom() {
                buf.cell_mut((x + dx, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(base_style);
                });
            }
        }
        col += 1;
    }
    // Fill remaining
    let start = col.saturating_sub(scroll_x);
    for dx in start..width {
        if x + dx < buf.area().right() && y < buf.area().bottom() {
            buf.cell_mut((x + dx, y)).map(|cell| cell.set_style(base_style));
        }
    }
}
```

- [ ] **Step 5: Implement StatefulWidget trait**

```rust
impl StatefulWidget for SideBySideDiff<'_> {
    type State = SideBySideDiffState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        if inner.width < 10 || inner.height < 2 {
            return;
        }

        // Build all rows from all files/hunks
        let mut all_rows: Vec<DiffRow> = Vec::new();
        state.hunk_offsets.clear();

        for file in self.diff_files {
            for hunk in &file.hunks {
                state.hunk_offsets.push(all_rows.len() as u16);
                let rows = pair_lines(hunk);
                all_rows.extend(rows);
            }
        }

        let line_no_width: u16 = 4;
        let separator_width: u16 = 1;
        let half_width = (inner.width - separator_width) / 2;
        let content_width = half_width.saturating_sub(line_no_width + 1); // +1 for space

        let visible_rows = inner.height as usize;
        let start = state.scroll_y as usize;

        for (i, row) in all_rows.iter().skip(start).take(visible_rows).enumerate() {
            let y = inner.y + i as u16;

            // Left line number
            if let Some(ln) = row.left_line_no {
                let ln_str = format!("{:>width$} ", ln, width = line_no_width as usize);
                buf.set_string(inner.x, y, &ln_str, Theme::diff_line_number());
            } else {
                let blank = " ".repeat(line_no_width as usize + 1);
                buf.set_string(inner.x, y, &blank, Theme::diff_line_number());
            }

            // Left content
            render_content_with_word_diff(
                buf,
                inner.x + line_no_width + 1,
                y,
                content_width,
                state.scroll_x,
                row.left_content.as_deref(),
                row.right_content.as_deref(),
                true,
                &row.left_kind,
            );

            // Separator
            let sep_x = inner.x + half_width;
            buf.set_string(sep_x, y, "│", Theme::border());

            // Right line number
            let right_x = sep_x + separator_width;
            if let Some(ln) = row.right_line_no {
                let ln_str = format!("{:>width$} ", ln, width = line_no_width as usize);
                buf.set_string(right_x, y, &ln_str, Theme::diff_line_number());
            } else {
                let blank = " ".repeat(line_no_width as usize + 1);
                buf.set_string(right_x, y, &blank, Theme::diff_line_number());
            }

            // Right content
            render_content_with_word_diff(
                buf,
                right_x + line_no_width + 1,
                y,
                content_width,
                state.scroll_x,
                row.left_content.as_deref(),
                row.right_content.as_deref(),
                false,
                &row.right_kind,
            );
        }
    }
}
```

- [ ] **Step 6: Write snapshot tests**

Add tests at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gitat_core::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_test_diff() -> Vec<DiffFile> {
        vec![DiffFile {
            old_path: "test.rs".to_string(),
            new_path: "test.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "    old_code();".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    new_code();".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "}".to_string(),
                        old_line_no: Some(3),
                        new_line_no: Some(3),
                    },
                ],
            }],
        }]
    }

    #[test]
    fn test_pair_lines_context() {
        let hunk = DiffHunk {
            old_start: 1, old_count: 1, new_start: 1, new_count: 1,
            lines: vec![DiffLine {
                kind: DiffLineKind::Context,
                content: "hello".to_string(),
                old_line_no: Some(1),
                new_line_no: Some(1),
            }],
        };
        let rows = pair_lines(&hunk);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].left_content.as_deref(), Some("hello"));
        assert_eq!(rows[0].right_content.as_deref(), Some("hello"));
    }

    #[test]
    fn test_pair_lines_change() {
        let hunk = DiffHunk {
            old_start: 1, old_count: 1, new_start: 1, new_count: 1,
            lines: vec![
                DiffLine { kind: DiffLineKind::Removed, content: "old".to_string(), old_line_no: Some(1), new_line_no: None },
                DiffLine { kind: DiffLineKind::Added, content: "new".to_string(), old_line_no: None, new_line_no: Some(1) },
            ],
        };
        let rows = pair_lines(&hunk);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].left_content.as_deref(), Some("old"));
        assert_eq!(rows[0].right_content.as_deref(), Some("new"));
    }

    #[test]
    fn test_render_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = make_test_diff();
        let mut state = SideBySideDiffState::new();

        terminal.draw(|f| {
            let widget = SideBySideDiff::new(&diff);
            f.render_stateful_widget(widget, f.area(), &mut state);
        }).unwrap();
    }

    #[test]
    fn test_hunk_navigation() {
        let mut state = SideBySideDiffState::new();
        state.hunk_offsets = vec![0, 10, 25];

        state.next_hunk();
        assert_eq!(state.current_hunk, 1);
        assert_eq!(state.scroll_y, 10);

        state.next_hunk();
        assert_eq!(state.current_hunk, 2);
        assert_eq!(state.scroll_y, 25);

        state.next_hunk(); // no more hunks
        assert_eq!(state.current_hunk, 2);

        state.prev_hunk();
        assert_eq!(state.current_hunk, 1);
        assert_eq!(state.scroll_y, 10);
    }
}
```

- [ ] **Step 7: Run tests**

```bash
cargo test -p gitat-ui side_by_side
```

Expected: All tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/gitat-ui/
git commit -m "feat(ui): implement side-by-side diff widget with word-level highlighting"
```

---

## Task 11: gitat-ui — Conflict Editor Widget

Custom widget for 2-way conflict comparison with editable RESULT area.

**Files:**
- Modify: `crates/gitat-ui/src/widgets/conflict_editor.rs`

- [ ] **Step 1: Write conflict editor state and types**

```rust
use gitat_core::conflict::{ConflictFile, ConflictRegion};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct ConflictEditorState {
    pub current_conflict: usize,
    pub total_conflicts: usize,
    pub resolved_count: usize,
    pub result_lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub editing: bool,
    pub scroll_y: u16,
}

impl ConflictEditorState {
    pub fn from_conflict_file(file: &ConflictFile) -> Self {
        let total = file.regions.iter().filter(|r| matches!(r, ConflictRegion::Conflict { .. })).count();

        // Initialize result with ours content
        let mut result_lines = Vec::new();
        for region in &file.regions {
            match region {
                ConflictRegion::Clean(lines) => result_lines.extend(lines.clone()),
                ConflictRegion::Conflict { ours, .. } => result_lines.extend(ours.clone()),
            }
        }

        Self {
            current_conflict: 0,
            total_conflicts: total,
            resolved_count: 0,
            result_lines,
            cursor_line: 0,
            cursor_col: 0,
            editing: false,
            scroll_y: 0,
        }
    }

    pub fn use_ours(&mut self, file: &ConflictFile) {
        self.apply_choice(file, true);
    }

    pub fn use_theirs(&mut self, file: &ConflictFile) {
        self.apply_choice(file, false);
    }

    fn apply_choice(&mut self, file: &ConflictFile, use_ours: bool) {
        let mut result = Vec::new();
        let mut conflict_idx = 0;

        for region in &file.regions {
            match region {
                ConflictRegion::Clean(lines) => result.extend(lines.clone()),
                ConflictRegion::Conflict { ours, theirs } => {
                    if conflict_idx == self.current_conflict {
                        if use_ours {
                            result.extend(ours.clone());
                        } else {
                            result.extend(theirs.clone());
                        }
                        self.resolved_count += 1;
                    } else {
                        result.extend(ours.clone()); // Keep ours as default for unresolved
                    }
                    conflict_idx += 1;
                }
            }
        }

        self.result_lines = result;
    }

    pub fn next_conflict(&mut self) {
        if self.current_conflict + 1 < self.total_conflicts {
            self.current_conflict += 1;
        }
    }

    pub fn prev_conflict(&mut self) {
        self.current_conflict = self.current_conflict.saturating_sub(1);
    }

    pub fn result_content(&self) -> String {
        self.result_lines.join("\n")
    }
}

pub struct ConflictEditor<'a> {
    file: &'a ConflictFile,
}

impl<'a> ConflictEditor<'a> {
    pub fn new(file: &'a ConflictFile) -> Self {
        Self { file }
    }
}
```

- [ ] **Step 2: Implement rendering**

```rust
impl StatefulWidget for ConflictEditor<'_> {
    type State = ConflictEditorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),     // header
                Constraint::Percentage(50), // ours | theirs
                Constraint::Percentage(50), // result
            ])
            .split(area);

        // Header
        let header = format!(
            " Conflict: {} — {}/{} resolved ",
            self.file.path, state.resolved_count, state.total_conflicts
        );
        buf.set_string(chunks[0].x, chunks[0].y, &header, Theme::diff_hunk_header());

        // Ours | Theirs split
        let compare = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Find current conflict's ours/theirs
        let mut conflict_idx = 0;
        let mut ours_text = String::new();
        let mut theirs_text = String::new();

        for region in &self.file.regions {
            if let ConflictRegion::Conflict { ours, theirs } = region {
                if conflict_idx == state.current_conflict {
                    ours_text = ours.join("\n");
                    theirs_text = theirs.join("\n");
                    break;
                }
                conflict_idx += 1;
            }
        }

        let ours_block = Block::default()
            .title(" OURS ")
            .borders(Borders::ALL)
            .border_style(Theme::conflict_ours());
        let ours_para = Paragraph::new(ours_text).block(ours_block).style(Theme::conflict_ours());
        ours_para.render(compare[0], buf);

        let theirs_block = Block::default()
            .title(" THEIRS ")
            .borders(Borders::ALL)
            .border_style(Theme::conflict_theirs());
        let theirs_para = Paragraph::new(theirs_text).block(theirs_block).style(Theme::conflict_theirs());
        theirs_para.render(compare[1], buf);

        // Result
        let result_block = Block::default()
            .title(" RESULT (o: ours, t: theirs, e: edit) ")
            .borders(Borders::ALL)
            .border_style(Theme::conflict_result());
        let result_text = state.result_lines.join("\n");
        let result_para = Paragraph::new(result_text).block(result_block).style(Theme::conflict_result());
        result_para.render(chunks[2], buf);
    }
}
```

- [ ] **Step 3: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gitat_core::conflict::{ConflictFile, ConflictRegion};

    fn make_test_conflict() -> ConflictFile {
        ConflictFile {
            path: "test.rs".to_string(),
            regions: vec![
                ConflictRegion::Clean(vec!["line1".to_string()]),
                ConflictRegion::Conflict {
                    ours: vec!["ours_line".to_string()],
                    theirs: vec!["theirs_line".to_string()],
                },
                ConflictRegion::Clean(vec!["line3".to_string()]),
            ],
        }
    }

    #[test]
    fn test_initial_result_uses_ours() {
        let file = make_test_conflict();
        let state = ConflictEditorState::from_conflict_file(&file);
        assert_eq!(state.result_lines, vec!["line1", "ours_line", "line3"]);
        assert_eq!(state.total_conflicts, 1);
    }

    #[test]
    fn test_use_theirs() {
        let file = make_test_conflict();
        let mut state = ConflictEditorState::from_conflict_file(&file);
        state.use_theirs(&file);
        assert_eq!(state.result_lines, vec!["line1", "theirs_line", "line3"]);
    }

    #[test]
    fn test_use_ours() {
        let file = make_test_conflict();
        let mut state = ConflictEditorState::from_conflict_file(&file);
        state.use_theirs(&file); // switch to theirs first
        state.use_ours(&file);
        assert_eq!(state.result_lines, vec!["line1", "ours_line", "line3"]);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p gitat-ui conflict
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gitat-ui/
git commit -m "feat(ui): implement conflict resolution editor widget"
```

---

## Task 12: gitat Binary — Main Entry Point & Event Loop

Wire everything together in the binary crate.

**Files:**
- Modify: `crates/gitat/src/main.rs`

- [ ] **Step 1: Implement main with terminal setup and event loop**

`crates/gitat/src/main.rs`:

```rust
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Tabs};

use gitat_core::runner::ProcessRunner;
use gitat_ui::app::{App, Mode, Tab};
use gitat_ui::event::handle_key;
use gitat_ui::theme::Theme;
use gitat_ui::views;
use gitat_ui::views::commit::render_commit_popup;

fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore();
        original_hook(panic_info);
    }));

    let repo_path = std::env::current_dir().context("failed to get current directory")?;
    let runner = ProcessRunner::new(repo_path);

    let mut terminal = ratatui::init();
    let mut app = App::new();
    app.refresh(&runner);

    let result = run_app(&mut terminal, &mut app, &runner);

    ratatui::restore();
    result
}

fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    runner: &ProcessRunner,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // tabs
                    Constraint::Min(0),     // main content
                    Constraint::Length(1),  // status bar
                ])
                .split(f.area());

            // Tab bar
            let tab_titles: Vec<Line> = [Tab::Status, Tab::Branches, Tab::Log, Tab::Stash]
                .iter()
                .map(|t| {
                    let style = if *t == app.tab {
                        Theme::tab_active()
                    } else {
                        Theme::tab_inactive()
                    };
                    Line::from(Span::styled(t.title(), style))
                })
                .collect();

            let tabs = Tabs::new(tab_titles)
                .select(match app.tab {
                    Tab::Status => 0,
                    Tab::Branches => 1,
                    Tab::Log => 2,
                    Tab::Stash => 3,
                })
                .highlight_style(Theme::tab_active());
            f.render_widget(tabs, chunks[0]);

            // Main content
            views::render_tab(f, app, chunks[1]);

            // Status bar
            let status_text = if let Some(ref msg) = app.status_message {
                msg.clone()
            } else {
                "j/k: move  h/l: panel  Tab: switch  s: stage  c: commit  p: push  ?: help  q: quit".to_string()
            };
            let status_bar = ratatui::widgets::Paragraph::new(status_text)
                .style(Theme::status_bar());
            f.render_widget(status_bar, chunks[2]);

            // Popups
            if matches!(app.mode, Mode::Commit { .. }) {
                render_commit_popup(f, app);
            }

            if matches!(app.mode, Mode::Help) {
                render_help_popup(f);
            }
        })?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key, runner);
            }
        }
    }

    Ok(())
}

fn render_help_popup(f: &mut ratatui::Frame) {
    let area = centered_rect(50, 60, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let help_text = vec![
        "j/k       Move up/down",
        "h/l       Switch panel",
        "Tab       Next tab",
        "Shift+Tab Previous tab",
        "Enter     View diff / expand",
        "s         Stage/unstage",
        "c         Commit",
        "p         Push",
        "P         Pull",
        "r         Refresh",
        "n/N       Next/prev hunk (diff)",
        "/         Search",
        "?         Toggle help",
        "q         Quit",
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());

    let paragraph = ratatui::widgets::Paragraph::new(help_text.join("\n")).block(block);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
```

- [ ] **Step 2: Verify build and run**

```bash
cargo build
cargo run -- 2>/dev/null || true  # Run briefly to test startup
```

Expected: Build succeeds. Binary starts and shows TUI (may exit immediately if no terminal).

- [ ] **Step 3: Commit**

```bash
git add crates/gitat/
git commit -m "feat: wire up main binary with event loop, tabs, and status bar"
```

---

## Task 13: Integration — Wire Diff Widget into Status View

Connect the side-by-side diff widget to the status view's detail panel.

**Files:**
- Modify: `crates/gitat-ui/src/views/status.rs`
- Modify: `crates/gitat-ui/src/app.rs`

- [ ] **Step 1: Add diff state to App**

In `app.rs`, add:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiffState;

// Add to App struct:
pub diff_state: SideBySideDiffState,

// In App::new():
diff_state: SideBySideDiffState::new(),
```

- [ ] **Step 2: Update status view to use SideBySideDiff**

Replace the placeholder in `render_detail()`:

```rust
use crate::widgets::side_by_side_diff::SideBySideDiff;

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.panel == Panel::Right {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.current_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let widget = SideBySideDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.diff_state);
    } else {
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let placeholder = Paragraph::new("Select a file and press Enter to view diff")
            .block(block);
        f.render_widget(placeholder, area);
    }
}
```

- [ ] **Step 3: Add diff scroll/hunk keys to event handler**

In `event.rs`, add to `handle_normal_mode`:

```rust
KeyCode::Char('n') if app.panel == Panel::Right => app.diff_state.next_hunk(),
KeyCode::Char('N') if app.panel == Panel::Right => app.diff_state.prev_hunk(),
KeyCode::Char('J') if app.panel == Panel::Right => app.diff_state.scroll_down(1),
KeyCode::Char('K') if app.panel == Panel::Right => app.diff_state.scroll_up(1),
KeyCode::Char('H') if app.panel == Panel::Right => app.diff_state.scroll_left(4),
KeyCode::Char('L') if app.panel == Panel::Right => app.diff_state.scroll_right(4),
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/
git commit -m "feat: integrate side-by-side diff widget into status view"
```

---

## Task 14: Integration — Wire Conflict Editor

Connect the conflict editor to the app flow.

**Files:**
- Modify: `crates/gitat-ui/src/event.rs`
- Modify: `crates/gitat-ui/src/app.rs`
- Modify: `crates/gitat/src/main.rs`

- [ ] **Step 1: Add conflict state to App**

In `app.rs`, add:

```rust
use crate::widgets::conflict_editor::ConflictEditorState;
use gitat_core::conflict::ConflictFile;

// Add to App struct:
pub conflict_state: Option<ConflictEditorState>,
pub conflict_file: Option<ConflictFile>,

// In App::new():
conflict_state: None,
conflict_file: None,
```

- [ ] **Step 2: Add conflict key handling in event.rs**

Add a new handler for Conflict mode:

```rust
fn handle_conflict_mode(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    let (state, file) = match (&mut app.conflict_state, &app.conflict_file) {
        (Some(state), Some(file)) => (state, file),
        _ => {
            app.mode = Mode::Normal;
            return;
        }
    };

    match key.code {
        KeyCode::Char('o') => state.use_ours(file),
        KeyCode::Char('t') => state.use_theirs(file),
        KeyCode::Char('n') => state.next_conflict(),
        KeyCode::Char('N') => state.prev_conflict(),
        KeyCode::Char('w') => {
            let content = state.result_content();
            let path = file.path.clone();
            if let Err(e) = gitat_core::conflict::resolve_file(runner, &path, &content) {
                app.set_status_message(format!("Resolve failed: {e}"));
            } else {
                app.set_status_message(format!("Resolved: {path}"));
                app.refresh(runner);
            }
            app.conflict_state = None;
            app.conflict_file = None;
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => {
            app.conflict_state = None;
            app.conflict_file = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}
```

Update `handle_key` to dispatch to `handle_conflict_mode`:

```rust
Mode::Conflict { .. } => handle_conflict_mode(app, key, runner),
```

- [ ] **Step 3: Render conflict editor in main.rs**

Add conflict editor rendering in the draw closure, after the main content:

```rust
if matches!(app.mode, Mode::Conflict { .. }) {
    if let (Some(ref file), Some(ref mut state)) = (&app.conflict_file, &mut app.conflict_state) {
        let editor = gitat_ui::widgets::conflict_editor::ConflictEditor::new(file);
        f.render_stateful_widget(editor, chunks[1], state);
    }
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/
git commit -m "feat: integrate conflict resolution editor into app flow"
```

---

## Task 15: Final Polish — .gitignore, Error Handling, README

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Update .gitignore**

Add `.superpowers/` to `.gitignore`.

- [ ] **Step 2: Run full test suite and build release**

```bash
cargo test
cargo build --release
```

Expected: All tests PASS, release build succeeds.

- [ ] **Step 3: Smoke test**

```bash
cd /tmp && git init test-repo && cd test-repo
echo "hello" > test.txt && git add . && git commit -m "init"
echo "world" >> test.txt
../../path/to/target/release/gitat
```

Verify: TUI starts, status shows modified file, diff displays correctly, `q` quits cleanly.

- [ ] **Step 4: Commit**

```bash
git add .gitignore
git commit -m "chore: update gitignore and final polish"
```
