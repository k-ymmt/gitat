use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel};
use crate::widgets::unified_diff::UnifiedDiffState;
use gitat_core::runner::CommandRunner;

pub(super) fn load_commit_preview_at(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) {
    let commit = match app.log_entries.get(log_entry_index) {
        Some(c) => c.clone(),
        None => return,
    };

    let first_parent = commit.parent_hashes.first().map(|s| s.as_str());
    let files =
        match gitat_core::commit_detail::get_commit_files(runner, &commit.hash, first_parent) {
            Ok(f) => f,
            Err(_) => return,
        };

    app.commit_detail_files = files;
    app.commit_detail_commit = Some(commit);
}

pub(super) fn load_commit_preview(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => return,
    };
    load_commit_preview_at(app, runner, idx);
}

pub(super) fn prepare_commit_detail(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) -> bool {
    let commit = match app.log_entries.get(log_entry_index) {
        Some(c) => c.clone(),
        None => return false,
    };

    let first_parent = commit.parent_hashes.first().map(|s| s.as_str());
    let files =
        match gitat_core::commit_detail::get_commit_files(runner, &commit.hash, first_parent) {
            Ok(f) => f,
            Err(e) => {
                app.set_status_message(format!("Failed to load commit files: {e}"));
                return false;
            }
        };

    app.commit_detail_commit = Some(commit);
    app.commit_detail_files = files;
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_panel = Panel::Left;
    app.commit_detail_diff_state = UnifiedDiffState::new();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    true
}

pub(super) fn enter_commit_detail_at(
    app: &mut App,
    runner: &dyn CommandRunner,
    log_entry_index: usize,
) {
    if prepare_commit_detail(app, runner, log_entry_index) {
        app.mode = Mode::CommitDetail;
    }
}

pub(super) fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) if i > 0 => i - 1, // offset: index 0 is uncommitted item
        _ => return,
    };
    enter_commit_detail_at(app, runner, idx);
}

fn load_commit_detail_diff(app: &mut App, runner: &dyn CommandRunner) {
    let commit = match &app.commit_detail_commit {
        Some(c) => c,
        None => return,
    };
    let idx = match app.commit_detail_file_state.selected() {
        Some(i) => i,
        None => return,
    };
    let file_entry = match app.commit_detail_files.get(idx) {
        Some(f) => f,
        None => return,
    };

    let parent = commit.parent_hashes.first().map(|s| s.as_str());
    match gitat_core::commit_detail::get_commit_file_diff(
        runner,
        &commit.hash,
        parent,
        &file_entry.path,
    ) {
        Ok(diff) => {
            app.commit_detail_diff = Some(diff);
            app.commit_detail_diff_state = UnifiedDiffState::new();
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

pub(super) fn handle_commit_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.pop_mode();
            // If stack was empty, pop_mode is no-op, fall back to Normal
            if matches!(app.mode, Mode::CommitDetail) {
                app.mode = Mode::Normal;
            }
            // Reset interactive state; keep data for preview
            app.commit_detail_file_state = ratatui::widgets::ListState::default();
            app.commit_detail_diff_state = UnifiedDiffState::new();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('h') => {
            app.commit_detail_panel = Panel::Left;
        }
        KeyCode::Char('l') => {
            app.commit_detail_panel = Panel::Right;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.commit_detail_panel == Panel::Left {
                let len = app.commit_detail_files.len();
                if len > 0 {
                    let i = match app.commit_detail_file_state.selected() {
                        Some(i) => (i + 1).min(len - 1),
                        None => 0,
                    };
                    app.commit_detail_file_state.select(Some(i));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail_diff_state.scroll_down(1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.commit_detail_panel == Panel::Left {
                if let Some(i) = app.commit_detail_file_state.selected() {
                    let next = if i == 0 { 0 } else { i - 1 };
                    app.commit_detail_file_state.select(Some(next));
                    load_commit_detail_diff(app, runner);
                }
            } else {
                app.commit_detail_diff_state.scroll_up(1);
            }
        }
        KeyCode::Char('J') => {
            app.commit_detail_diff_state.scroll_down(1);
        }
        KeyCode::Char('K') => {
            app.commit_detail_diff_state.scroll_up(1);
        }
        KeyCode::Char('H') => {
            app.commit_detail_diff_state.scroll_left(4);
        }
        KeyCode::Char('L') => {
            app.commit_detail_diff_state.scroll_right(4);
        }
        KeyCode::Char('n') => {
            app.commit_detail_diff_state.next_hunk();
        }
        KeyCode::Char('N') => {
            app.commit_detail_diff_state.prev_hunk();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use super::load_commit_preview;
    use crate::app::{App, Mode, Panel, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    #[test]
    fn test_load_commit_preview_filtered_mode_index_zero() {
        use super::super::load_log_preview;

        let mut app = App::new();
        app.log_entries = vec![
            gitat_core::log::CommitInfo {
                hash: "aaa111".to_string(),
                short_hash: "aaa".to_string(),
                author: "Alice".to_string(),
                date: "2026-01-01".to_string(),
                message: "first".to_string(),
                refs: vec![],
                parent_hashes: vec!["p1".to_string()],
            },
            gitat_core::log::CommitInfo {
                hash: "bbb222".to_string(),
                short_hash: "bbb".to_string(),
                author: "Bob".to_string(),
                date: "2026-01-02".to_string(),
                message: "second".to_string(),
                refs: vec![],
                parent_hashes: vec!["p2".to_string()],
            },
        ];
        // Simulate filtered mode: only the second commit matches
        app.filtered_log_indices = Some(vec![1]);
        app.log_list_state.select(Some(0)); // first item in filtered list

        let runner = MockRunner::new().with_response(
            "diff-tree --no-commit-id -r --name-status p2 bbb222",
            "M\tlib.rs\n",
        );

        load_log_preview(&mut app, &runner);

        // Should load the second commit (index 1 in log_entries)
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_commit.as_ref().unwrap().hash, "bbb222");
        assert_eq!(app.commit_detail_files.len(), 1);
    }

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_load_commit_preview_populates_data() {
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
        app.log_list_state.select(Some(1)); // index 1 = first commit

        let runner = MockRunner::new().with_response(
            "diff-tree --no-commit-id -r --name-status parent1 abc123",
            "M\tsrc/main.rs\n",
        );

        load_commit_preview(&mut app, &runner);

        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
        assert_eq!(app.commit_detail_files[0].path, "src/main.rs");
    }

    #[test]
    fn test_enter_log_tab_enters_commit_detail_mode() {
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
        app.log_list_state.select(Some(1)); // index 0 is uncommitted item, index 1 is first commit

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status parent1 abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
    }

    #[test]
    fn test_esc_from_commit_detail_returns_to_search() {
        let mut app = App::new();
        // Simulate: was in Search, pushed to CommitDetail
        app.mode = Mode::CommitDetail;
        app.mode_stack = vec![Mode::Search { query: "test".into() }];
        app.filtered_log_indices = Some(vec![0, 2]);
        app.pre_search_cursor = Some(5);

        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

        assert!(matches!(app.mode, Mode::Search { ref query } if query == "test"));
        assert_eq!(app.filtered_log_indices, Some(vec![0, 2]));
        assert_eq!(app.pre_search_cursor, Some(5));
    }

    #[test]
    fn test_esc_from_commit_detail_falls_back_to_normal() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        // Empty stack — entered from Normal mode directly
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_esc_from_commit_detail_preserves_preview_data() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        app.commit_detail_commit = Some(gitat_core::log::CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            author: "Test".to_string(),
            date: "2026-04-04".to_string(),
            message: "test".to_string(),
            refs: vec![],
            parent_hashes: vec![],
        });
        app.commit_detail_files = vec![gitat_core::commit_detail::CommitFileEntry {
            path: "src/main.rs".to_string(),
            status: gitat_core::commit_detail::FileChangeStatus::Modified,
        }];

        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);

        assert_eq!(app.mode, Mode::Normal);
        // Data should be preserved for preview
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
    }

    #[test]
    fn test_commit_detail_panel_switch() {
        let mut app = App::new();
        app.mode = Mode::CommitDetail;
        app.commit_detail_panel = Panel::Left;
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('l')), &runner);
        assert_eq!(app.commit_detail_panel, Panel::Right);
        handle_key(&mut app, mock_key(KeyCode::Char('h')), &runner);
        assert_eq!(app.commit_detail_panel, Panel::Left);
    }
}
