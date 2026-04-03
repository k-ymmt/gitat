# gitat — Git TUI Client Design

## Overview

gitat is a terminal-based Git client built with Rust and ratatui 0.30.x. It aims to provide a lazygit-equivalent feature set with improved UI/UX, specifically focusing on high-quality side-by-side diff display and intuitive conflict resolution with inline editing.

## Goals

- Comprehensive Git TUI client (status, log, diff, stage/unstage, commit, branch, push/pull)
- Side-by-side diff with word-level highlighting as primary differentiator
- 2-way conflict resolution editor with inline editing capability
- Vim-like keybindings for efficient navigation
- Clean separation between Git operations and UI

## Non-Goals (MVP)

- Interactive rebase UI
- Stash management (tab placeholder only)
- Git submodule support
- Configuration file / custom keybindings
- Syntax highlighting in diff

## Architecture

### Workspace Structure

```
gitat/
├── Cargo.toml              # workspace root
├── crates/
│   ├── gitat-core/         # Git operations (CLI-independent)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── repository.rs   # Repository info
│   │       ├── status.rs       # git status parsing
│   │       ├── log.rs          # git log parsing
│   │       ├── diff.rs         # git diff parsing
│   │       ├── branch.rs       # Branch operations
│   │       ├── commit.rs       # Commit operations
│   │       ├── remote.rs       # push/pull
│   │       ├── stage.rs        # stage/unstage
│   │       └── conflict.rs     # Conflict detection & resolution
│   │
│   ├── gitat-ui/            # TUI layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs          # App state & screen transitions
│   │       ├── event.rs        # Key event handling
│   │       ├── theme.rs        # Color theme
│   │       ├── widgets/        # Custom widgets
│   │       │   ├── mod.rs
│   │       │   ├── side_by_side_diff.rs
│   │       │   └── conflict_editor.rs
│   │       └── views/          # Screen views
│   │           ├── mod.rs
│   │           ├── status.rs
│   │           ├── log.rs
│   │           ├── diff.rs
│   │           ├── branches.rs
│   │           ├── commit.rs
│   │           └── conflict.rs
│   │
│   └── gitat/               # Binary entry point
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
```

**Dependency direction:** `gitat` → `gitat-ui` → `gitat-core`. The core crate has no ratatui dependency.

## gitat-core Design

### Command Execution

```rust
pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
}

pub struct ProcessRunner {
    repo_path: PathBuf,
}

pub struct MockRunner {
    responses: HashMap<String, String>,
}
```

`ProcessRunner` executes git commands via `std::process::Command`. `MockRunner` is used in tests to return fixture strings.

### Core Types

```rust
// status.rs
pub struct StatusEntry {
    pub path: String,
    pub index_status: FileStatus,
    pub worktree_status: FileStatus,
}

pub enum FileStatus {
    Modified, Added, Deleted, Renamed, Copied, Untracked, Unmodified,
}

// log.rs
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub refs: Vec<String>,
    pub parent_hashes: Vec<String>,
}

// diff.rs
pub struct DiffFile {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

pub struct DiffLine {
    pub kind: DiffLineKind, // Added, Removed, Context
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

// branch.rs
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub last_commit: String,
}

// conflict.rs
pub struct ConflictFile {
    pub path: String,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
}
```

### Operations

| Module | Key Methods |
|--------|------------|
| `status` | `get_status() -> Vec<StatusEntry>` |
| `log` | `get_log(limit, branch) -> Vec<CommitInfo>` |
| `diff` | `get_diff(staged: bool) -> Vec<DiffFile>`, `get_diff_for_file(path)` |
| `branch` | `list_branches()`, `create_branch()`, `checkout()`, `delete_branch()` |
| `commit` | `commit(message)`, `amend(message)` |
| `stage` | `stage_file(path)`, `unstage_file(path)`, `stage_hunk(...)` |
| `remote` | `push(remote, branch)`, `pull(remote, branch)`, `fetch()` |
| `conflict` | `get_conflicts() -> Vec<ConflictFile>`, `resolve_file(path, content)` |

### Error Type

```rust
pub enum GitError {
    CommandFailed { command: String, stderr: String, exit_code: i32 },
    ParseError(String),
    NotARepository,
    IoError(std::io::Error),
}
```

## gitat-ui Design

### Layout: 2-Column + Tab

```
[Status] [Branches] [Log] [Stash]
┌─ Files ────────┬─ Detail ──────────────────────────────┐
│ Staged (2)     │ src/app.rs — side-by-side diff        │
│ ▸ M src/app.rs │  OLD              │  NEW              │
│   A src/new.rs │  10 let app =     │  10 let app =     │
│                │  11-app.run();    │  11+let config =   │
│ Modified (1)   │                   │  12+app.run_with() │
│   M Cargo.toml │  12 println!      │  13 println!      │
└────────────────┴──────────────────────────────────────┘
j/k: move  h/l: panel  Tab: tab  s: stage  c: commit  ?: help
```

Left panel: file list (grouped by staged/unstaged/untracked). Right panel: detail view (diff, branch info, commit details depending on context).

Top tabs switch between Status, Branches, Log, and Stash views.

### App State

```rust
pub enum Tab {
    Status,
    Branches,
    Log,
    Stash,
}

pub enum Mode {
    Normal,
    Commit { message: String },
    Conflict { file: String },
    Search { query: String },
    Help,
}

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub should_quit: bool,
    pub file_list_state: ListState,
    pub diff_scroll: (u16, u16),
    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log: Vec<CommitInfo>,
    pub current_diff: Option<Vec<DiffFile>>,
}
```

### Keybindings

| Key | Normal Mode |
|-----|------------|
| `j/k` | Move up/down in list |
| `h/l` | Switch left/right panel |
| `Tab/Shift+Tab` | Switch tab |
| `Enter` | Show diff for selected file |
| `s` | Stage/unstage toggle |
| `c` | Enter commit mode |
| `p` | Push |
| `P` | Pull |
| `b` | Create branch |
| `d` | Delete branch |
| `/` | Enter search mode |
| `?` | Help popup |
| `q` | Quit |
| `r` | Refresh |

Commit mode: `Enter` to confirm, `Esc` to cancel.

## Side-by-side Diff Widget

The primary differentiator of gitat.

### Features

- **Line alignment** — deleted and added lines are aligned horizontally; insertions/deletions are padded with empty lines
- **Word-level diff** — changed words within a line are highlighted with a distinct background color (using the `similar` crate)
- **Independent line numbers** — each side shows its own line numbers
- **Hunk navigation** — `n/N` to jump to next/previous hunk
- **Scrolling** — `j/k` for vertical, `H/L` for horizontal (long lines)
- **Context folding** — unchanged regions are collapsed; `Enter` to expand

### Implementation

The widget is a custom `StatefulWidget` implementation. It takes `Vec<DiffFile>` as input and maintains scroll position and current hunk index as state. Line pairing logic aligns old/new lines by computing edit sequences per hunk.

## Conflict Resolution Editor

### Layout

```
                Conflict: src/config.rs — 2/5 resolved
┌─ OURS (main) ─────────────┬─ THEIRS (feature/auth) ──────┐
│ 14 pub struct Config {     │ 14 pub struct Config {        │
│ 15   pub timeout: u64,     │ 15   pub timeout: Duration,   │
│ 16   pub retries: u32,     │ 16   pub max_retries: u32,    │
│                            │ 17   pub auth_token: String,   │
│ 17 }                       │ 18 }                          │
├─ RESULT (editable) ────────┴──────────────────────────────┤
│ 14 pub struct Config {                                     │
│ 15   pub timeout: Duration, █                              │
│ 16   pub max_retries: u32,                                 │
│ 17   pub auth_token: String,                               │
│ 18 }                                                       │
└────────────────────────────────────────────────────────────┘
o: use ours  t: use theirs  e: edit result  n/N: nav  w: save
```

### Workflow

1. Select a conflict file from the list → editor opens
2. Top: OURS (left) and THEIRS (right) 2-way comparison
3. Bottom: RESULT area (initially populated with OURS content)
4. `o` to adopt ours, `t` to adopt theirs → updates RESULT
5. `e` to enter RESULT area for direct text editing (Vim-like)
6. `n/N` to navigate between conflict markers in the same file
7. `w` to save and auto-stage (`git add`)

## Testing Strategy

### gitat-core

- `CommandRunner` trait allows swapping `ProcessRunner` for `MockRunner` in tests
- Git CLI output strings stored as fixtures for parser unit tests
- Each parser module (status, log, diff, branch, conflict) has dedicated tests

### gitat-ui

- `TestBackend` from ratatui for rendering capture
- `insta` for snapshot tests — detects UI regressions as diffs
- State transition unit tests: key input → App state change (no rendering needed)

## Error Handling

- **gitat-core**: returns `Result<T, GitError>`, never panics
- **gitat-ui**: displays core errors in a status bar at the bottom (auto-clears after 3 seconds)
- **Panic hook**: `std::panic::set_hook` restores terminal state before printing panic info
- **Fatal errors** (not a repository, git not installed): prints message and exits at startup

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal events & control |
| `anyhow` | Error handling (binary crate) |
| `thiserror` | Error type definitions (core crate) |
| `insta` | Snapshot testing |
| `similar` | Word-level diff computation |
