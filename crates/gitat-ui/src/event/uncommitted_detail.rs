use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::side_by_side_diff::SideBySideDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn load_uncommitted_preview(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(entry) = app.status.first() {
        let staged = entry.is_staged();
        if let Ok(diff) = gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
            app.current_diff = Some(diff);
        }
    } else {
        app.current_diff = None;
    }
}

pub(super) fn enter_uncommitted_detail(app: &mut App, runner: &dyn CommandRunner) {
    app.uncommitted_list_state = ratatui::widgets::ListState::default();
    app.panel = Panel::Left;
    app.diff_state = SideBySideDiffState::new();
    app.current_diff = None;

    if !app.status.is_empty() {
        app.uncommitted_list_state.select(Some(0));
        load_uncommitted_diff(app, runner);
    }
    app.mode = Mode::UncommittedDetail;
}

fn load_uncommitted_diff(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.uncommitted_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let staged = entry.is_staged();
    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.current_diff = Some(diff);
            app.diff_state = SideBySideDiffState::new();
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_uncommitted_detail(
    app: &mut App,
    key: KeyEvent,
    runner: &dyn CommandRunner,
) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.current_diff = None;
            app.diff_state = SideBySideDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.panel == Panel::Left {
                let len = app.status.len();
                if len > 0 {
                    let i = match app.uncommitted_list_state.selected() {
                        Some(i) => (i + 1).min(len - 1),
                        None => 0,
                    };
                    app.uncommitted_list_state.select(Some(i));
                    load_uncommitted_diff(app, runner);
                }
            } else {
                app.diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.panel == Panel::Left {
                if let Some(i) = app.uncommitted_list_state.selected() {
                    let next = if i == 0 { 0 } else { i - 1 };
                    app.uncommitted_list_state.select(Some(next));
                    load_uncommitted_diff(app, runner);
                }
            } else {
                app.diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('s') => {
            match app.panel {
                Panel::Left => super::staging::stage_or_unstage(app, runner),
                Panel::Right => super::staging::stage_or_unstage_hunk(app, runner),
            }
        }
        KeyCode::Char('c') => {
            app.return_to_uncommitted_detail = true;
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('J') => {
            app.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.diff_state.prev_hunk();
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use super::load_uncommitted_preview;
    use crate::app::{App, Mode, Panel, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_load_uncommitted_preview_populates_diff() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Modified,
            worktree_status: gitat_core::status::FileStatus::Unmodified,
        }];

        let runner = MockRunner::new()
            .with_response("diff --cached -- src/main.rs", "");

        load_uncommitted_preview(&mut app, &runner);

        assert!(app.current_diff.is_some());
    }

    #[test]
    fn test_load_uncommitted_preview_empty_status() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.status = vec![];

        let runner = MockRunner::new();
        load_uncommitted_preview(&mut app, &runner);

        assert!(app.current_diff.is_none());
    }

    #[test]
    fn test_esc_from_uncommitted_detail_returns_to_normal() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_uncommitted_detail_panel_switch() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.panel, Panel::Left);
    }

    #[test]
    fn test_uncommitted_detail_c_enters_commit_mode() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('c')), &runner);
        assert!(matches!(app.mode, Mode::Commit { .. }));
        assert!(app.return_to_uncommitted_detail);
    }

    #[test]
    fn test_uncommitted_detail_q_quits() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('q')), &runner);
        assert!(app.should_quit);
    }

    #[test]
    fn test_enter_on_index_0_enters_uncommitted_detail() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.log_list_state.select(Some(0));

        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::UncommittedDetail));
    }

    #[test]
    fn test_enter_on_index_1_enters_commit_detail() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.log_entries = vec![gitat_core::log::CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            author: "Test".to_string(),
            date: "2026-04-04".to_string(),
            message: "test commit".to_string(),
            refs: vec![],
            parent_hashes: vec!["parent1".to_string()],
        }];
        app.log_list_state.select(Some(1));

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
    }
}
