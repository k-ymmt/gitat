use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, Panel, Tab};
use gitat_core::runner::CommandRunner;

pub(super) fn handle_normal(app: &mut App, key: KeyEvent, runner: &dyn CommandRunner) {
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
        KeyCode::Char('n') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.next_hunk();
        }
        KeyCode::Char('N') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.prev_hunk();
        }
        KeyCode::Char('J') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.scroll_down(1);
        }
        KeyCode::Char('K') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.scroll_up(1);
        }
        KeyCode::Char('H') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.scroll_left(4);
        }
        KeyCode::Char('L') if app.uncommitted.panel == Panel::Right => {
            app.uncommitted.diff_state.scroll_right(4);
        }
        KeyCode::Char('l') => {
            app.uncommitted.panel = Panel::Right;
        }
        KeyCode::Char('h') => {
            app.uncommitted.panel = Panel::Left;
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
            if app.tab == Tab::Log {
                super::load_log_preview(app, runner);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let state = app.current_list_state_mut();
            if let Some(i) = state.selected() {
                let next = if i == 0 { 0 } else { i - 1 };
                state.select(Some(next));
            }
            if app.tab == Tab::Log {
                super::load_log_preview(app, runner);
            }
        }
        KeyCode::Char('c') => {
            app.mode = Mode::Commit {
                message: String::new(),
            };
        }
        KeyCode::Char('p') => match gitat_core::branch::current_branch(runner) {
            Ok(branch) => {
                if let Err(e) = gitat_core::remote::push(runner, "origin", &branch) {
                    app.status_bar.set(format!("Push failed: {e}"));
                } else {
                    app.status_bar.set(format!("Pushed to origin/{branch}"));
                }
            }
            Err(e) => app.status_bar.set(format!("Push failed: {e}")),
        },
        KeyCode::Char('P') => match gitat_core::branch::current_branch(runner) {
            Ok(branch) => {
                if let Err(e) = gitat_core::remote::pull(runner, "origin", &branch) {
                    app.status_bar.set(format!("Pull failed: {e}"));
                } else {
                    app.status_bar.set(format!("Pulled from origin/{branch}"));
                    app.refresh(runner);
                }
            }
            Err(e) => app.status_bar.set(format!("Pull failed: {e}")),
        },
        KeyCode::Char('b') => {
            // Placeholder: branch creation requires user input (not yet implemented)
            app.status_bar.set("Branch creation: not yet implemented");
        }
        KeyCode::Char('d') => {
            if app.tab == Tab::Branches {
                delete_selected_branch(app, runner);
            }
        }
        KeyCode::Char('/') => {
            app.search.pre_search_cursor = app.log_list_state.selected();
            app.mode = Mode::Search {
                query: String::new(),
            };
        }
        KeyCode::Char('r') => {
            app.refresh(runner);
            app.status_bar.set("Refreshed");
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
        }
        KeyCode::Enter => {
            if app.tab == Tab::Log {
                match app.log_list_state.selected() {
                    Some(0) => super::uncommitted_detail::enter_uncommitted_detail(app, runner),
                    Some(_) => super::commit_detail::enter_commit_detail(app, runner),
                    None => {}
                }
            } else {
                super::staging::load_diff_for_selected(app, runner);
            }
        }
        _ => {}
    }
}

fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::Branches => app.branches.len(),
        Tab::Log => 1 + app.log_entries.len(),
        Tab::Stash => 0,
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
        app.status_bar.set("Cannot delete current branch");
        return;
    }
    match gitat_core::branch::delete_branch(runner, &branch.name) {
        Ok(()) => {
            app.refresh(runner);
            app.status_bar
                .set(format!("Deleted branch '{}'", branch.name));
        }
        Err(e) => {
            app.status_bar.set(format!("Delete branch failed: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use crate::app::{App, Mode, Tab};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_slash_saves_cursor_and_enters_search() {
        let mut app = App::new();
        app.log_list_state.select(Some(3));
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('/')), &runner);
        assert!(matches!(app.mode, Mode::Search { ref query } if query.is_empty()));
        assert_eq!(app.search.pre_search_cursor, Some(3));
    }

    #[test]
    fn test_j_on_log_tab_loads_commit_preview() {
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
        // No selection yet; first j press selects index 0 (uncommitted)
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(0));

        // Second j press selects index 1 (first commit) and loads preview
        let runner = MockRunner::new().with_response(
            "diff-tree --no-commit-id -r --name-status parent1 abc123",
            "M\tsrc/main.rs\n",
        );

        handle_key(&mut app, mock_key(KeyCode::Char('j')), &runner);
        assert_eq!(app.log_list_state.selected(), Some(1));
        assert!(app.commit_detail.commit.is_some());
        assert_eq!(app.commit_detail.files.len(), 1);
    }
}
