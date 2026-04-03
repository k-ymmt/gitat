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
        if let Ok(log) = gitat_core::log::get_log(runner, 100, None) {
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
