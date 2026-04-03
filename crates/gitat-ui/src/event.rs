use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel, Tab};
use gitat_core::runner::CommandRunner;

pub fn handle_key(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match app.mode {
        Mode::Normal => handle_normal(app, key, runner),
        Mode::Commit { .. } => handle_commit(app, key, runner),
        Mode::Help => handle_help(app, key),
        Mode::Search { .. } => handle_search(app, key),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => handle_commit_detail(app, key, runner),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.tab = app.tab.next();
        }
        KeyCode::BackTab => {
            app.tab = app.tab.prev();
        }
        // Diff navigation keys (uppercase, right panel only) — must be checked before lowercase h/l
        KeyCode::Char('n') if app.panel == Panel::Right => {
            app.diff_state.next_hunk();
        }
        KeyCode::Char('N') if app.panel == Panel::Right => {
            app.diff_state.prev_hunk();
        }
        KeyCode::Char('J') if app.panel == Panel::Right => {
            app.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') if app.panel == Panel::Right => {
            app.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') if app.panel == Panel::Right => {
            app.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') if app.panel == Panel::Right => {
            app.diff_state.scroll_right(4);
        }
        KeyCode::Char('l') => {
            app.panel = Panel::Right;
        }
        KeyCode::Char('h') => {
            app.panel = Panel::Left;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let len = list_len(app);
            if len > 0 {
                let state = app.current_list_state_mut();
                let i = match state.selected() {
                    Some(i) => (i + 1).min(len - 1),
                    None => 0,
                };
                state.select(Some(i));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let state = app.current_list_state_mut();
            if let Some(i) = state.selected() {
                let next = if i == 0 { 0 } else { i - 1 };
                state.select(Some(next));
            }
        }
        KeyCode::Char('s') => {
            if app.tab == Tab::Status {
                stage_or_unstage(app, runner);
            }
        }
        KeyCode::Char('c') => {
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('p') => {
            match gitat_core::branch::current_branch(runner) {
                Ok(branch) => {
                    if let Err(e) = gitat_core::remote::push(runner, "origin", &branch) {
                        app.set_status_message(format!("Push failed: {e}"));
                    } else {
                        app.set_status_message(format!("Pushed to origin/{branch}"));
                    }
                }
                Err(e) => app.set_status_message(format!("Push failed: {e}")),
            }
        }
        KeyCode::Char('P') => {
            match gitat_core::branch::current_branch(runner) {
                Ok(branch) => {
                    if let Err(e) = gitat_core::remote::pull(runner, "origin", &branch) {
                        app.set_status_message(format!("Pull failed: {e}"));
                    } else {
                        app.set_status_message(format!("Pulled from origin/{branch}"));
                        app.refresh(runner);
                    }
                }
                Err(e) => app.set_status_message(format!("Pull failed: {e}")),
            }
        }
        KeyCode::Char('b') => {
            // Placeholder: branch creation requires user input (not yet implemented)
            app.set_status_message("Branch creation: not yet implemented");
        }
        KeyCode::Char('d') => {
            if app.tab == Tab::Branches {
                delete_selected_branch(app, runner);
            }
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search {
                query: String::new(),
            };
        }
        KeyCode::Char('r') => {
            app.refresh(runner);
            app.set_status_message("Refreshed");
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        KeyCode::Enter => {
            if app.tab == Tab::Log {
                enter_commit_detail(app, runner);
            } else {
                load_diff_for_selected(app, runner);
            }
        }
        _ => {}
    }
}

fn handle_commit(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
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
                    app.mode = Mode::Normal;
                    app.set_status_message("Committed successfully");
                    app.refresh(runner);
                }
                Err(e) => {
                    app.set_status_message(format!("Commit failed: {e}"));
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

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            if let Mode::Search { query } = &mut app.mode {
                query.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Mode::Search { query } = &mut app.mode {
                query.push(c);
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

fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::Status => app.status.len(),
        Tab::Branches => app.branches.len(),
        Tab::Log => app.log_entries.len(),
        Tab::Stash => 0,
    }
}

fn stage_or_unstage(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    use gitat_core::status::FileStatus;
    let is_staged = !matches!(
        entry.index_status,
        FileStatus::Unmodified | FileStatus::Untracked
    );

    let result = if is_staged {
        gitat_core::stage::unstage_file(runner, &entry.path)
    } else {
        gitat_core::stage::stage_file(runner, &entry.path)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage failed: {e}"));
        }
    }
}

fn delete_selected_branch(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.branches_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let branch = match app.branches.get(idx) {
        Some(b) => b.clone(),
        None => return,
    };
    if branch.is_current {
        app.set_status_message("Cannot delete current branch");
        return;
    }
    match gitat_core::branch::delete_branch(runner, &branch.name) {
        Ok(()) => {
            app.refresh(runner);
            app.set_status_message(format!("Deleted branch '{}'", branch.name));
        }
        Err(e) => {
            app.set_status_message(format!("Delete branch failed: {e}"));
        }
    }
}

fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if app.tab != Tab::Status {
        return;
    }
    let idx = match app.status_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    use gitat_core::status::FileStatus;
    let staged = !matches!(
        entry.index_status,
        FileStatus::Unmodified | FileStatus::Untracked
    );

    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.current_diff = Some(diff);
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

fn enter_commit_detail(app: &mut App, runner: &dyn CommandRunner) {
    let idx = match app.log_list_state.selected() {
        Some(i) => i,
        None => return,
    };
    let commit = match app.log_entries.get(idx) {
        Some(c) => c.clone(),
        None => return,
    };

    let files = match gitat_core::commit_detail::get_commit_files(runner, &commit.hash) {
        Ok(f) => f,
        Err(e) => {
            app.set_status_message(format!("Failed to load commit files: {e}"));
            return;
        }
    };

    app.commit_detail_commit = Some(commit);
    app.commit_detail_files = files;
    app.commit_detail_file_state = ratatui::widgets::ListState::default();
    app.commit_detail_panel = Panel::Left;
    app.commit_detail_diff_state = crate::widgets::side_by_side_diff::SideBySideDiffState::new();
    if !app.commit_detail_files.is_empty() {
        app.commit_detail_file_state.select(Some(0));
        load_commit_detail_diff(app, runner);
    }
    app.mode = Mode::CommitDetail;
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
            app.commit_detail_diff_state = crate::widgets::side_by_side_diff::SideBySideDiffState::new();
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

fn handle_commit_detail(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.commit_detail_commit = None;
            app.commit_detail_files.clear();
            app.commit_detail_file_state = ratatui::widgets::ListState::default();
            app.commit_detail_diff = None;
            app.commit_detail_diff_state = crate::widgets::side_by_side_diff::SideBySideDiffState::new();
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
    use super::*;
    use crossterm::event::KeyModifiers;
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
        app.log_list_state.select(Some(0));

        let runner = MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\n",
            )
            .with_response("diff parent1..abc123 -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Enter), &runner);
        assert!(matches!(app.mode, Mode::CommitDetail));
        assert!(app.commit_detail_commit.is_some());
        assert_eq!(app.commit_detail_files.len(), 1);
    }

    #[test]
    fn test_esc_from_commit_detail_returns_to_normal() {
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
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.commit_detail_commit.is_none());
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
