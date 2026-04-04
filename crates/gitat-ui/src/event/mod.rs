mod commit_detail;
mod normal;
mod staging;
mod uncommitted_detail;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode};
use gitat_core::runner::CommandRunner;

pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match app.mode {
        Mode::Normal => normal::handle_normal(app, key, runner),
        Mode::Commit { .. } => handle_commit(app, key, runner),
        Mode::Help => handle_help(app, key),
        Mode::Search { .. } => handle_search(app, key, runner),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => commit_detail::handle_commit_detail(app, key, runner),
        Mode::UncommittedDetail => uncommitted_detail::handle_uncommitted_detail(app, key, runner),
    }
}

pub fn load_log_preview(app: &mut App, runner: &dyn CommandRunner) {
    if let Some(ref indices) = app.filtered_log_indices {
        // In filtered mode: resolve through filtered_log_indices
        if let Some(sel) = app.log_list_state.selected()
            && let Some(&log_idx) = indices.get(sel)
        {
            commit_detail::load_commit_preview_at(app, runner, log_idx);
        }
    } else {
        match app.log_list_state.selected() {
            Some(0) => uncommitted_detail::load_uncommitted_preview(app, runner),
            Some(_) => commit_detail::load_commit_preview(app, runner),
            None => {}
        }
    }
}

fn handle_commit(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            if app.return_to_uncommitted_detail {
                app.mode = Mode::UncommittedDetail;
                app.return_to_uncommitted_detail = false;
            } else {
                app.mode = Mode::Normal;
            }
        }
        KeyCode::Enter => {
            let message = match &app.mode {
                Mode::Commit { message } => message.clone(),
                _ => return,
            };
            if message.is_empty() {
                app.set_status_message("Commit message cannot be empty");
                return;
            }
            match gitat_core::commit::commit(runner, &message) {
                Ok(()) => {
                    if app.return_to_uncommitted_detail {
                        app.mode = Mode::UncommittedDetail;
                        app.return_to_uncommitted_detail = false;
                    } else {
                        app.mode = Mode::Normal;
                    }
                    app.set_status_message("Committed successfully");
                    app.refresh(runner);
                }
                Err(e) => {
                    app.set_status_message(format!("Commit failed: {e}"));
                    app.return_to_uncommitted_detail = false;
                    app.mode = Mode::Normal;
                }
            }
        }
        KeyCode::Backspace => {
            if let Mode::Commit { message } = &mut app.mode {
                message.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Commit { message } = &mut app.mode {
                message.push(c);
            }
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            if let Some(cursor) = app.pre_search_cursor {
                app.log_list_state.select(Some(cursor));
            }
            app.filtered_log_indices = None;
            app.pre_search_cursor = None;
            app.mode = Mode::Normal;
            app.mode_stack.clear();
        }
        KeyCode::Enter => {
            let original_index = app.log_list_state.selected().and_then(|sel| {
                app.filtered_log_indices
                    .as_ref()
                    .and_then(|indices| indices.get(sel).copied())
            });
            if let Some(idx) = original_index
                && commit_detail::prepare_commit_detail(app, runner, idx)
            {
                app.push_mode(Mode::CommitDetail);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.filtered_log_indices.as_ref().map_or(0, |v| v.len());
            if max > 0 {
                let current = app.log_list_state.selected().unwrap_or(0);
                let next = (current + 1).min(max - 1);
                app.log_list_state.select(Some(next));
            }
            load_log_preview(app, runner);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let current = app.log_list_state.selected().unwrap_or(0);
            app.log_list_state.select(Some(current.saturating_sub(1)));
            load_log_preview(app, runner);
        }
        KeyCode::Backspace => {
            if let Mode::Search { query } = &mut app.mode {
                query.pop();
                let q = query.clone();
                app.update_search_filter(&q);
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Search { query } = &mut app.mode {
                query.push(c);
                let q = query.clone();
                app.update_search_filter(&q);
            }
        }
        _ => {}
    }
}

fn handle_conflict(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
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
        KeyCode::Esc | KeyCode::Char('q') => {
            app.conflict_state = None;
            app.conflict_file = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use gitat_core::runner::MockRunner;

    use crate::app::{Panel, Tab};

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
        assert_eq!(app.tab, Tab::Log);
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
        app.mode = Mode::Commit {
            message: String::new(),
        };
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_search_typing_filters_log() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix bug".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "add feature".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.mode = Mode::Search {
            query: String::new(),
        };
        let runner = MockRunner::new();

        // Type 'f' — both match ("fix" and "feature")
        handle_key(&mut app, mock_key(KeyCode::Char('f')), &runner);
        assert!(matches!(&app.mode, Mode::Search { query } if query == "f"));
        assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));

        // Type 'i' — only "fix bug" matches
        handle_key(&mut app, mock_key(KeyCode::Char('i')), &runner);
        assert!(matches!(&app.mode, Mode::Search { query } if query == "fi"));
        assert_eq!(app.filtered_log_indices, Some(vec![0]));

        // Backspace — back to "f", both match again
        handle_key(&mut app, mock_key(KeyCode::Backspace), &runner);
        assert!(matches!(&app.mode, Mode::Search { query } if query == "f"));
        assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));
    }

    #[test]
    fn test_search_enter_opens_commit_detail() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa111".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "first".into(),
                refs: vec![],
                parent_hashes: vec!["p1".into()],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb222".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "second".into(),
                refs: vec![],
                parent_hashes: vec!["p2".into()],
            },
        ];
        app.pre_search_cursor = Some(0);
        app.mode = Mode::Search { query: "second".into() };
        app.update_search_filter("second");

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status p2 bbb222",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff p2..bbb222 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);

        assert_eq!(app.mode, Mode::CommitDetail);
        assert_eq!(app.mode_stack.len(), 1);
        assert!(matches!(&app.mode_stack[0], Mode::Search { query } if query == "second"));
        assert_eq!(app.filtered_log_indices, Some(vec![1]));
        assert_eq!(app.pre_search_cursor, Some(0));
        assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");
    }

    #[test]
    fn test_search_enter_noop_on_empty_results() {
        let mut app = App::new();
        app.log_entries = vec![];
        app.mode = Mode::Search { query: "nothing".into() };
        app.filtered_log_indices = Some(vec![]);
        app.log_list_state.select(None);
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::Search { .. }));
    }

    #[test]
    fn test_search_esc_restores_cursor() {
        let mut app = App::new();
        app.pre_search_cursor = Some(5);
        app.mode = Mode::Search {
            query: "test".into(),
        };
        app.filtered_log_indices = Some(vec![0, 2]);
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.filtered_log_indices, None);
        assert_eq!(app.log_list_state.selected(), Some(5));
    }

    #[test]
    fn test_search_esc_clears_mode_stack() {
        let mut app = App::new();
        app.mode = Mode::Search { query: "test".into() };
        app.pre_search_cursor = Some(3);
        app.filtered_log_indices = Some(vec![0]);
        app.mode_stack = vec![Mode::Normal]; // leftover from some transition
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

        assert_eq!(app.mode, Mode::Normal);
        assert!(app.mode_stack.is_empty());
        assert_eq!(app.filtered_log_indices, None);
    }

    #[test]
    fn test_search_j_moves_cursor_down() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix bug".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "fix typo".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.mode = Mode::Search { query: "fix".into() };
        app.update_search_filter("fix");
        // filtered_log_indices = Some([0, 1]), selection = 0
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(1));
        // Verify still in search mode
        assert!(matches!(app.mode, Mode::Search { ref query } if query == "fix"));
    }

    #[test]
    fn test_search_k_moves_cursor_up() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix bug".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "fix typo".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.mode = Mode::Search { query: "fix".into() };
        app.update_search_filter("fix");
        app.log_list_state.select(Some(1)); // start at second item
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Char('k')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(0));
    }

    #[test]
    fn test_search_j_clamps_at_end() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.mode = Mode::Search { query: "fix".into() };
        app.update_search_filter("fix");
        // Only one result, selection at 0
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(0)); // clamped
    }

    #[test]
    fn test_search_k_clamps_at_start() {
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix".into(),
                refs: vec![],
                parent_hashes: vec![],
            },
        ];
        app.mode = Mode::Search { query: "fix".into() };
        app.update_search_filter("fix");
        let runner = MockRunner::new();

        handle_key(&mut app, mock_key(KeyCode::Char('k')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(0)); // clamped at 0
    }

    #[test]
    fn test_search_to_commit_detail_round_trip() {
        // Full flow: Normal -> Search -> navigate -> Enter -> CommitDetail -> Esc -> Search -> Esc -> Normal
        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa111".into(),
                short_hash: "aaa".into(),
                author: "Alice".into(),
                date: "2026-01-01".into(),
                message: "fix bug".into(),
                refs: vec![],
                parent_hashes: vec!["p1".into()],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb222".into(),
                short_hash: "bbb".into(),
                author: "Bob".into(),
                date: "2026-01-02".into(),
                message: "fix typo".into(),
                refs: vec![],
                parent_hashes: vec!["p2".into()],
            },
        ];
        app.log_list_state.select(Some(1)); // some position in Normal mode

        let runner_search = MockRunner::new();

        // Step 1: Enter search mode
        handle_key(&mut app, mock_key(KeyCode::Char('/')), &runner_search);
        assert!(matches!(app.mode, Mode::Search { .. }));
        assert_eq!(app.pre_search_cursor, Some(1));

        // Step 2: Type "fix" — both match
        handle_key(&mut app, mock_key(KeyCode::Char('f')), &runner_search);
        handle_key(&mut app, mock_key(KeyCode::Char('i')), &runner_search);
        handle_key(&mut app, mock_key(KeyCode::Char('x')), &runner_search);
        assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));
        assert_eq!(app.log_list_state.selected(), Some(0));

        // Step 3: j to move to second result
        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner_search);
        assert_eq!(app.log_list_state.selected(), Some(1));

        // Step 4: Enter to open CommitDetail for second result (bbb222)
        let runner_detail = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status p2 bbb222",
                "M\tlib.rs\n",
            )
            .with_response("diff p2..bbb222 -- lib.rs", "");
        handle_key(&mut app, mock_key(KeyCode::Enter), &runner_detail);
        assert_eq!(app.mode, Mode::CommitDetail);
        assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");

        // Step 5: Esc from CommitDetail -> back to Search
        let runner_back = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner_back);
        assert!(matches!(app.mode, Mode::Search { ref query } if query == "fix"));
        assert_eq!(app.filtered_log_indices, Some(vec![0, 1]));

        // Step 6: Esc from Search -> back to Normal
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner_back);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.filtered_log_indices, None);
        assert_eq!(app.log_list_state.selected(), Some(1)); // restored
        assert!(app.mode_stack.is_empty());
    }
}
