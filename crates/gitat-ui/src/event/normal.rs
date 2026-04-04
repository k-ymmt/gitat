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
            // Staging is handled in UncommittedDetail mode, not in Normal mode
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
