use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::unified_diff::UnifiedDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn load_uncommitted_preview(_app: &mut App, _runner: &dyn CommandRunner) {
    // Preview only shows file list from app.status (already loaded by refresh).
    // No additional data loading needed.
}

pub(super) fn enter_uncommitted_detail(app: &mut App, runner: &dyn CommandRunner) {
    app.uncommitted_list_state = ratatui::widgets::ListState::default();
    app.panel = Panel::Left;
    app.diff_state = UnifiedDiffState::new();
    app.current_diff = None;
    app.rebuild_uncommitted_file_map();

    // Select the first file entry (skip section headers)
    if let Some(first) = app.uncommitted_file_map.iter().position(|x| x.is_some()) {
        app.uncommitted_list_state.select(Some(first));
        load_uncommitted_diff(app, runner);
    }
    app.mode = Mode::UncommittedDetail;
}

fn load_uncommitted_diff(app: &mut App, runner: &dyn CommandRunner) {
    let visual_idx = match app.uncommitted_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let (status_idx, staged) = match app.uncommitted_file_map.get(visual_idx) {
        Some(Some(info)) => *info,
        _ => return,
    };
    let entry = match app.status.get(status_idx) {
        Some(e) => e.clone(),
        None => return,
    };
    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.current_diff = Some(diff);
            app.diff_state = UnifiedDiffState::new();
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
            // Reset interactive state only; keep diff data for preview
            app.diff_state = UnifiedDiffState::new();
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
                let len = app.uncommitted_file_map.len();
                if len > 0 {
                    let current = app.uncommitted_list_state.selected().unwrap_or(0);
                    // Find next file entry (skip section headers)
                    let next = app
                        .uncommitted_file_map
                        .iter()
                        .enumerate()
                        .skip(current + 1)
                        .find(|(_, x)| x.is_some())
                        .map(|(i, _)| i);
                    if let Some(next) = next {
                        app.uncommitted_list_state.select(Some(next));
                        load_uncommitted_diff(app, runner);
                    }
                }
            } else {
                app.diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.panel == Panel::Left {
                if let Some(current) = app.uncommitted_list_state.selected()
                    && current > 0
                {
                    // Find previous file entry (skip section headers)
                    let prev = app.uncommitted_file_map[..current]
                        .iter()
                        .rposition(|x| x.is_some());
                    if let Some(prev) = prev {
                        app.uncommitted_list_state.select(Some(prev));
                        load_uncommitted_diff(app, runner);
                    }
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
    use crate::app::{App, Mode, Panel, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
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
    fn test_esc_from_uncommitted_detail_preserves_diff() {
        let mut app = App::new();
        app.tab = Tab::Log;
        app.mode = Mode::UncommittedDetail;
        app.current_diff = Some(vec![]);

        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

        assert_eq!(app.mode, Mode::Normal);
        // Diff data should be preserved for preview
        assert!(app.current_diff.is_some());
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
                "diff-tree --no-commit-id -r --name-status parent1 abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
    }
}
