use crate::app::{App, Mode};
use crate::widgets::unified_diff::UnifiedDiffState;
use gitat_core::runner::CommandRunner;

/// Returns (status_index, is_in_staged_section) for the currently selected file.
fn selected_file_info(app: &App) -> Option<(usize, bool)> {
    let visual_idx = app.uncommitted_list_state.selected()?;
    app.uncommitted_file_map.get(visual_idx).copied().flatten()
}

pub(super) fn stage_or_unstage(app: &mut App, runner: &dyn CommandRunner) {
    let (idx, is_staged) = match selected_file_info(app) {
        Some(info) => info,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let result = if is_staged {
        gitat_core::stage::unstage_file(runner, &entry.path)
    } else {
        gitat_core::stage::stage_file(runner, &entry.path)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
            app.rebuild_uncommitted_file_map();
            app.clamp_uncommitted_selection();
            // Reload diff for the new selection, or clear if nothing selected
            app.current_diff = None;
            app.diff_state = UnifiedDiffState::new();
            load_diff_for_selected(app, runner);
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage failed: {e}"));
        }
    }
}

pub(super) fn stage_or_unstage_hunk(app: &mut App, runner: &dyn CommandRunner) {
    let (idx, is_staged) = match selected_file_info(app) {
        Some(info) => info,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    let diff_files = match &app.current_diff {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };

    let diff_file = match diff_files
        .iter()
        .find(|f| f.new_path == entry.path || f.old_path == entry.path)
    {
        Some(f) => f.clone(),
        None => return,
    };

    let hunk_index = app.diff_state.current_hunk;

    let result = if is_staged {
        gitat_core::stage::unstage_hunk(runner, &diff_file, hunk_index)
    } else {
        gitat_core::stage::stage_hunk(runner, &diff_file, hunk_index)
    };

    match result {
        Ok(()) => {
            app.refresh(runner);
            app.rebuild_uncommitted_file_map();
            app.clamp_uncommitted_selection();
            // After staging, show remaining unstaged diff; after unstaging, show remaining staged diff
            match gitat_core::diff::get_diff_for_file(runner, &entry.path, !is_staged) {
                Ok(diff) => {
                    if diff.is_empty() || diff.iter().all(|f| f.hunks.is_empty()) {
                        app.current_diff = None;
                        app.diff_state = UnifiedDiffState::new();
                    } else {
                        // Clamp current_hunk
                        let total_hunks: usize = diff.iter().map(|f| f.hunks.len()).sum();
                        if app.diff_state.current_hunk >= total_hunks {
                            app.diff_state.current_hunk = total_hunks.saturating_sub(1);
                        }
                        app.current_diff = Some(diff);
                    }
                }
                Err(e) => {
                    app.set_status_message(format!("Failed to reload diff: {e}"));
                    app.current_diff = None;
                    app.diff_state = UnifiedDiffState::new();
                }
            }
        }
        Err(e) => {
            app.set_status_message(format!("Stage/unstage hunk failed: {e}"));
        }
    }
}

pub(super) fn load_diff_for_selected(app: &mut App, runner: &dyn CommandRunner) {
    if !matches!(app.mode, Mode::UncommittedDetail) {
        return;
    }
    let (idx, staged) = match selected_file_info(app) {
        Some(info) => info,
        None => return,
    };
    let entry = match app.status.get(idx) {
        Some(e) => e.clone(),
        None => return,
    };

    match gitat_core::diff::get_diff_for_file(runner, &entry.path, staged) {
        Ok(diff) => {
            app.current_diff = Some(diff);
        }
        Err(e) => {
            app.set_status_message(format!("Failed to load diff: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::handle_key;
    use crate::app::{App, Mode, Panel};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use gitat_core::runner::MockRunner;

    fn mock_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_s_in_right_panel_calls_stage_hunk() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Right;

        // Set up a status entry (unstaged modified file)
        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Unmodified,
            worktree_status: gitat_core::status::FileStatus::Modified,
        }];
        app.rebuild_uncommitted_file_map();
        // Visual index 0 = "Modified" header, 1 = the file
        app.uncommitted_list_state.select(Some(1));

        // Set up current diff with one hunk
        app.current_diff = Some(vec![gitat_core::diff::DiffFile {
            old_path: "src/main.rs".to_string(),
            new_path: "src/main.rs".to_string(),
            hunks: vec![gitat_core::diff::DiffHunk {
                old_start: 1,
                old_count: 2,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Context,
                        content: "line1".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Added,
                        content: "new_line".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    gitat_core::diff::DiffLine {
                        kind: gitat_core::diff::DiffLineKind::Context,
                        content: "line2".to_string(),
                        old_line_no: Some(2),
                        new_line_no: Some(3),
                    },
                ],
            }],
        }]);
        app.diff_state.current_hunk = 0;

        // LOG_FORMAT = "%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e"
        let log_key = "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e";
        let runner = MockRunner::new()
            .with_response("apply --cached", "")
            .with_response("status --porcelain=v1", "")
            .with_response("branch -v --no-color", "")
            .with_response(log_key, "")
            .with_response("diff -- src/main.rs", "")
            .with_response("diff --cached -- src/main.rs", "");

        handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);

        // After staging the only hunk, diff should be reloaded (now empty)
        assert!(
            app.status_message.is_none()
                || !app.status_message.as_ref().unwrap().contains("failed")
        );
    }

    #[test]
    fn test_s_in_left_panel_still_stages_file() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Left;
        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Unmodified,
            worktree_status: gitat_core::status::FileStatus::Modified,
        }];
        app.rebuild_uncommitted_file_map();
        app.uncommitted_list_state.select(Some(1));

        let runner = MockRunner::new()
            .with_response("add -- src/main.rs", "")
            .with_response("status --porcelain=v1", "")
            .with_response("branch -v --no-color", "")
            .with_response(
                "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e",
                "",
            );

        handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);
        // Should not error — file-level stage_file was called
        assert!(app.status_message.is_none());
    }

    #[test]
    fn test_s_in_uncommitted_detail_stages_file() {
        let mut app = App::new();
        app.mode = Mode::UncommittedDetail;
        app.panel = Panel::Left;
        app.status = vec![gitat_core::status::StatusEntry {
            path: "src/main.rs".to_string(),
            index_status: gitat_core::status::FileStatus::Unmodified,
            worktree_status: gitat_core::status::FileStatus::Modified,
        }];
        app.rebuild_uncommitted_file_map();
        app.uncommitted_list_state.select(Some(1));

        let runner = MockRunner::new()
            .with_response("add -- src/main.rs", "")
            .with_response("status --porcelain=v1", "")
            .with_response("branch -v --no-color", "")
            .with_response(
                "log --max-count=100 --format=%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e",
                "",
            );

        handle_key(&mut app, mock_key(KeyCode::Char('s')), &runner);
        assert!(app.status_message.is_none());
    }
}
