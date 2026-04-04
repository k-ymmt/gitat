use std::time::{Duration, Instant};

use gitat_core::branch::BranchInfo;
use gitat_core::commit_detail::CommitFileEntry;
use gitat_core::conflict::ConflictFile;
use gitat_core::diff::DiffFile;
use gitat_core::log::CommitInfo;
use gitat_core::runner::CommandRunner;
use gitat_core::status::StatusEntry;
use ratatui::widgets::ListState;

use crate::widgets::conflict_editor::ConflictEditorState;
use crate::widgets::side_by_side_diff::SideBySideDiffState;

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

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub panel: Panel,
    pub should_quit: bool,
    pub uncommitted_list_state: ListState,
    pub log_list_state: ListState,
    pub branches_list_state: ListState,
    pub diff_state: SideBySideDiffState,
    pub status: Vec<StatusEntry>,
    pub branches: Vec<BranchInfo>,
    pub log_entries: Vec<CommitInfo>,
    pub current_diff: Option<Vec<DiffFile>>,
    pub status_message: Option<String>,
    pub status_message_set_at: Option<Instant>,
    pub conflict_state: Option<ConflictEditorState>,
    pub conflict_file: Option<ConflictFile>,
    pub commit_detail_commit: Option<CommitInfo>,
    pub commit_detail_files: Vec<CommitFileEntry>,
    pub commit_detail_file_state: ListState,
    pub commit_detail_panel: Panel,
    pub commit_detail_diff: Option<Vec<DiffFile>>,
    pub commit_detail_diff_state: SideBySideDiffState,
    pub return_to_uncommitted_detail: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: Tab::Log,
            mode: Mode::Normal,
            panel: Panel::Left,
            should_quit: false,
            uncommitted_list_state: ListState::default(),
            log_list_state: ListState::default(),
            branches_list_state: ListState::default(),
            diff_state: SideBySideDiffState::new(),
            status: Vec::new(),
            branches: Vec::new(),
            log_entries: Vec::new(),
            current_diff: None,
            status_message: None,
            status_message_set_at: None,
            conflict_state: None,
            conflict_file: None,
            commit_detail_commit: None,
            commit_detail_files: Vec::new(),
            commit_detail_file_state: ListState::default(),
            commit_detail_panel: Panel::Left,
            commit_detail_diff: None,
            commit_detail_diff_state: SideBySideDiffState::new(),
            return_to_uncommitted_detail: false,
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

    pub fn set_status_message(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_message_set_at = Some(Instant::now());
    }

    pub fn clear_expired_status_message(&mut self) {
        if let Some(set_at) = self.status_message_set_at
            && set_at.elapsed() > Duration::from_secs(3)
        {
            self.status_message = None;
            self.status_message_set_at = None;
        }
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
        assert_eq!(app.panel, Panel::Left);
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
        app.set_status_message("hello");
        app.clear_expired_status_message();
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_status_message_cleared_after_expiry() {
        let mut app = App::new();
        app.set_status_message("hello");
        app.status_message_set_at = Some(Instant::now() - Duration::from_secs(4));
        app.clear_expired_status_message();
        assert!(app.status_message.is_none());
        assert!(app.status_message_set_at.is_none());
    }
}
