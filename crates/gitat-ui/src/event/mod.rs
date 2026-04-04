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
        Mode::Search { .. } => handle_search(app, key),
        Mode::Conflict { .. } => handle_conflict(app, key, runner),
        Mode::CommitDetail => commit_detail::handle_commit_detail(app, key, runner),
        Mode::UncommittedDetail => uncommitted_detail::handle_uncommitted_detail(app, key, runner),
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
        app.mode = Mode::Commit { message: String::new() };
        let runner = MockRunner::new();
        handle_key(&mut app, mock_key(KeyCode::Esc), &runner);
        assert_eq!(app.mode, Mode::Normal);
    }
}
