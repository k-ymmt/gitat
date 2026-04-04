use std::time::{Duration, Instant};

use gitat_core::branch::BranchInfo;
use gitat_core::commit_detail::CommitFileEntry;
use gitat_core::conflict::ConflictFile;
use gitat_core::diff::DiffFile;
use gitat_core::log::CommitInfo;
use gitat_core::runner::CommandRunner;
use gitat_core::status::{FileStatus, StatusEntry};
use ratatui::widgets::ListState;

use crate::widgets::conflict_editor::ConflictEditorState;
use crate::widgets::unified_diff::UnifiedDiffState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Log,
    Branches,
    Stash,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Log => Tab::Branches,
            Tab::Branches => Tab::Stash,
            Tab::Stash => Tab::Log,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Log => Tab::Stash,
            Tab::Branches => Tab::Log,
            Tab::Stash => Tab::Branches,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tab::Log => "Log",
            Tab::Branches => "Branches",
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
    CommitDetail,
    UncommittedDetail,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Left,
    Right,
}

pub struct CommitDetailState {
    pub commit: Option<CommitInfo>,
    pub files: Vec<CommitFileEntry>,
    pub file_state: ListState,
    pub panel: Panel,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
}

impl CommitDetailState {
    pub fn new() -> Self {
        Self {
            commit: None,
            files: Vec::new(),
            file_state: ListState::default(),
            panel: Panel::Left,
            diff: None,
            diff_state: UnifiedDiffState::new(),
        }
    }
}

impl Default for CommitDetailState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UncommittedState {
    pub list_state: ListState,
    /// Maps visual list index to (status_index, is_in_staged_section).
    /// None for section headers.
    pub file_map: Vec<Option<(usize, bool)>>,
    pub diff: Option<Vec<DiffFile>>,
    pub diff_state: UnifiedDiffState,
    pub panel: Panel,
}

impl UncommittedState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            file_map: Vec::new(),
            diff: None,
            diff_state: UnifiedDiffState::new(),
            panel: Panel::Left,
        }
    }
}

impl Default for UncommittedState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SearchState {
    /// Original cursor position before entering search mode, for Esc restoration.
    pub pre_search_cursor: Option<usize>,
    /// Indices into `log_entries` matching the current search query.
    /// `None` = no filter (normal display). `Some(vec)` = filtered view.
    pub filtered_log_indices: Option<Vec<usize>>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            pre_search_cursor: None,
            filtered_log_indices: None,
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConflictResolveState {
    pub editor_state: Option<ConflictEditorState>,
    pub file: Option<ConflictFile>,
}

impl ConflictResolveState {
    pub fn new() -> Self {
        Self {
            editor_state: None,
            file: None,
        }
    }
}

impl Default for ConflictResolveState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StatusBar {
    pub message: Option<String>,
    pub set_at: Option<Instant>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            message: None,
            set_at: None,
        }
    }

    pub fn set(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
        self.set_at = Some(Instant::now());
    }

    pub fn clear_if_expired(&mut self) {
        if let Some(set_at) = self.set_at
            && set_at.elapsed() > STATUS_MESSAGE_TIMEOUT
        {
            self.message = None;
            self.set_at = None;
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(3);

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub mode_stack: Vec<Mode>,
    pub should_quit: bool,
    pub return_to_uncommitted_detail: bool,
    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log_entries: Vec<CommitInfo>,
    pub log_list_state: ListState,
    pub branches_list_state: ListState,
    pub uncommitted: UncommittedState,
    pub commit_detail: CommitDetailState,
    pub search: SearchState,
    pub conflict: ConflictResolveState,
    pub status_bar: StatusBar,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: Tab::Log,
            mode: Mode::Normal,
            mode_stack: Vec::new(),
            should_quit: false,
            return_to_uncommitted_detail: false,
            status: Vec::new(),
            branches: Vec::new(),
            log_entries: Vec::new(),
            log_list_state: ListState::default(),
            branches_list_state: ListState::default(),
            uncommitted: UncommittedState::new(),
            commit_detail: CommitDetailState::new(),
            search: SearchState::new(),
            conflict: ConflictResolveState::new(),
            status_bar: StatusBar::new(),
        }
    }

    pub fn push_mode(&mut self, mode: Mode) {
        let current = std::mem::replace(&mut self.mode, mode);
        self.mode_stack.push(current);
    }

    pub fn pop_mode(&mut self) {
        if let Some(prev) = self.mode_stack.pop() {
            self.mode = prev;
        }
    }

    pub fn current_list_state_mut(&mut self) -> &mut ListState {
        match self.tab {
            Tab::Branches => &mut self.branches_list_state,
            Tab::Log => &mut self.log_list_state,
            Tab::Stash => &mut self.log_list_state, // fallback
        }
    }

    pub fn refresh(&mut self, runner: &dyn CommandRunner) {
        if let Ok(status) = gitat_core::status::get_status(runner) {
            self.status = status;
        }
        if let Ok(branches) = gitat_core::branch::list_branches(runner) {
            self.branches = branches;
        }
        if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
            self.log_entries = log;
        }
    }

    pub fn refresh_status_and_log(&mut self, runner: &dyn CommandRunner) {
        if let Ok(status) = gitat_core::status::get_status(runner) {
            self.status = status;
        }
        if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
            self.log_entries = log;
        }
    }

    pub fn rebuild_uncommitted_file_map(&mut self) {
        let mut map: Vec<Option<(usize, bool)>> = Vec::new();

        // Staged section (must match render_uncommitted_file_list order)
        let staged: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
            })
            .map(|(i, _)| i)
            .collect();

        if !staged.is_empty() {
            map.push(None); // header
            for idx in staged {
                map.push(Some((idx, true)));
            }
        }

        // Modified (unstaged) section — includes files that are also staged (e.g. MM)
        let modified: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.worktree_status != FileStatus::Unmodified
                    && e.worktree_status != FileStatus::Untracked
            })
            .map(|(i, _)| i)
            .collect();

        if !modified.is_empty() {
            map.push(None); // header
            for idx in modified {
                map.push(Some((idx, false)));
            }
        }

        // Untracked section
        let untracked: Vec<usize> = self
            .status
            .iter()
            .enumerate()
            .filter(|(_, e)| e.index_status == FileStatus::Untracked)
            .map(|(i, _)| i)
            .collect();

        if !untracked.is_empty() {
            map.push(None); // header
            for idx in untracked {
                map.push(Some((idx, false)));
            }
        }

        self.uncommitted.file_map = map;
    }

    pub fn clamp_uncommitted_selection(&mut self) {
        if self.uncommitted.file_map.is_empty() {
            self.uncommitted.list_state.select(None);
            return;
        }

        let current = match self.uncommitted.list_state.selected() {
            Some(i) => i,
            None => {
                if let Some(pos) = self.uncommitted.file_map.iter().position(|x| x.is_some()) {
                    self.uncommitted.list_state.select(Some(pos));
                }
                return;
            }
        };

        // If current is valid and points to a file, keep it
        if current < self.uncommitted.file_map.len()
            && self.uncommitted.file_map[current].is_some()
        {
            return;
        }

        // Find nearest valid file entry (forward first, then backward)
        let forward = self
            .uncommitted
            .file_map
            .iter()
            .enumerate()
            .skip(current)
            .find(|(_, x)| x.is_some())
            .map(|(i, _)| i);
        let backward = self.uncommitted.file_map
            [..current.min(self.uncommitted.file_map.len())]
            .iter()
            .rposition(|x| x.is_some());

        self.uncommitted.list_state.select(forward.or(backward));
    }

    pub fn update_search_filter(&mut self, query: &str) {
        if query.is_empty() {
            self.search.filtered_log_indices = None;
            return;
        }
        let query_lower = query.to_lowercase();
        let indices: Vec<usize> = self
            .log_entries
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.message.to_lowercase().contains(&query_lower)
                    || c.short_hash.to_lowercase().contains(&query_lower)
                    || c.author.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();
        self.search.filtered_log_indices = Some(indices);
        self.log_list_state.select(Some(0));
    }

}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_cycling() {
        assert_eq!(Tab::Log.next(), Tab::Branches);
        assert_eq!(Tab::Stash.next(), Tab::Log);
        assert_eq!(Tab::Log.prev(), Tab::Stash);
        assert_eq!(Tab::Branches.prev(), Tab::Log);
    }

    #[test]
    fn test_app_initial_state() {
        let app = App::new();
        assert_eq!(app.tab, Tab::Log);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.uncommitted.panel, Panel::Left);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_uncommitted_detail_mode_exists() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        assert_eq!(app.mode, Mode::UncommittedDetail);
    }

    #[test]
    fn test_status_message_not_cleared_before_expiry() {
        let mut app = App::new();
        app.status_bar.set("hello");
        app.status_bar.clear_if_expired();
        assert!(app.status_bar.message.is_some());
    }

    #[test]
    fn test_status_message_cleared_after_expiry() {
        let mut app = App::new();
        app.status_bar.set("hello");
        app.status_bar.set_at = Some(Instant::now() - Duration::from_secs(4));
        app.status_bar.clear_if_expired();
        assert!(app.status_bar.message.is_none());
        assert!(app.status_bar.set_at.is_none());
    }

    #[test]
    fn test_app_search_fields_initial_state() {
        let app = App::new();
        assert_eq!(app.search.pre_search_cursor, None);
        assert_eq!(app.search.filtered_log_indices, None);
    }

    #[test]
    fn test_update_search_filter_matches_message() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix login bug".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "add tests".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
            gitat_core::log::CommitInfo {
                hash: "ccc".into(),
                short_hash: "ccc".into(),
                author: "Alice".into(),
                date: "2026-01-03".into(),
                message: "update readme".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.log_list_state.select(Some(2));

        // Filter by "fix" — only first entry matches
        app.update_search_filter("fix");
        assert_eq!(app.search.filtered_log_indices, Some(vec![0]));
        assert_eq!(app.log_list_state.selected(), Some(0));

        // Filter by "alice" (case-insensitive) — matches author in entries 0 and 2
        app.update_search_filter("alice");
        assert_eq!(app.search.filtered_log_indices, Some(vec![0, 2]));

        // Filter by "bbb" — matches short_hash
        app.update_search_filter("bbb");
        assert_eq!(app.search.filtered_log_indices, Some(vec![1]));

        // Empty query — clears filter
        app.update_search_filter("");
        assert_eq!(app.search.filtered_log_indices, None);
    }

    #[test]
    fn test_push_mode_saves_current_to_stack() {
        let mut app = App::new();
        assert_eq!(app.mode, Mode::Normal);
        app.push_mode(Mode::Search { query: "test".into() });
        assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
        assert_eq!(app.mode_stack.len(), 1);
        assert_eq!(app.mode_stack[0], Mode::Normal);
    }

    #[test]
    fn test_pop_mode_restores_previous() {
        let mut app = App::new();
        app.push_mode(Mode::Search { query: "test".into() });
        app.push_mode(Mode::CommitDetail);
        assert_eq!(app.mode, Mode::CommitDetail);
        app.pop_mode();
        assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
        app.pop_mode();
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.mode_stack.is_empty());
    }

    #[test]
    fn test_pop_mode_noop_on_empty_stack() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        app.pop_mode();
        assert_eq!(app.mode, Mode::CommitDetail);
    }

    #[test]
    fn test_refresh_status_and_log_updates_status_and_log() {
        use gitat_core::runner::MockRunner;

        let runner = MockRunner::new()
            .with_response("status --porcelain=v1", " M src/main.rs\n")
            .with_response(
                "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e",
                "abc123\x1fabc\x1f\x1fHEAD -> main\x1fAuthor\x1f2026-04-04\x1fInitial commit\x1e",
            );

        let mut app = App::new();
        app.refresh_status_and_log(&runner);

        assert_eq!(app.status.len(), 1);
        assert_eq!(app.log_entries.len(), 1);
        // branches should remain unchanged (not refreshed)
        assert!(app.branches.is_empty());
    }
}
