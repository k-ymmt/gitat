use gitat_core::conflict::{ConflictFile, ConflictRegion};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};

use crate::theme::Theme;

pub struct ConflictEditorState {
    pub current_conflict: usize,
    pub total_conflicts: usize,
    pub resolved_count: usize,
    pub result_lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub editing: bool,
    pub scroll_y: u16,
}

impl ConflictEditorState {
    pub fn from_conflict_file(file: &ConflictFile) -> Self {
        let total_conflicts = file
            .regions
            .iter()
            .filter(|r| matches!(r, ConflictRegion::Conflict { .. }))
            .count();

        let mut state = Self {
            current_conflict: 0,
            total_conflicts,
            resolved_count: 0,
            result_lines: Vec::new(),
            cursor_line: 0,
            cursor_col: 0,
            editing: false,
            scroll_y: 0,
        };

        // Initialize result with ours content
        state.apply_choice(file, true);
        state
    }

    pub fn use_ours(&mut self, file: &ConflictFile) {
        self.apply_choice(file, true);
    }

    pub fn use_theirs(&mut self, file: &ConflictFile) {
        self.apply_choice(file, false);
    }

    /// Rebuild result_lines by walking all regions; for the current conflict index,
    /// use the chosen side (ours if `use_ours` is true, theirs otherwise).
    fn apply_choice(&mut self, file: &ConflictFile, use_ours: bool) {
        let mut result = Vec::new();
        let mut conflict_idx = 0;

        for region in &file.regions {
            match region {
                ConflictRegion::Clean(lines) => {
                    result.extend(lines.iter().cloned());
                }
                ConflictRegion::Conflict { ours, theirs } => {
                    if conflict_idx == self.current_conflict {
                        if use_ours {
                            result.extend(ours.iter().cloned());
                        } else {
                            result.extend(theirs.iter().cloned());
                        }
                    } else {
                        // For other conflicts, keep what was previously chosen (ours as default)
                        result.extend(ours.iter().cloned());
                    }
                    conflict_idx += 1;
                }
            }
        }

        self.result_lines = result;
    }

    pub fn next_conflict(&mut self) {
        if self.current_conflict + 1 < self.total_conflicts {
            self.current_conflict += 1;
        }
    }

    pub fn prev_conflict(&mut self) {
        if self.current_conflict > 0 {
            self.current_conflict -= 1;
        }
    }

    pub fn result_content(&self) -> String {
        self.result_lines.join("\n")
    }
}

pub struct ConflictEditor<'a> {
    file: &'a ConflictFile,
}

impl<'a> ConflictEditor<'a> {
    pub fn new(file: &'a ConflictFile) -> Self {
        Self { file }
    }
}

impl StatefulWidget for ConflictEditor<'_> {
    type State = ConflictEditorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Split vertically: header (1), top half (ours/theirs), bottom half (result)
        let [header_area, top_area, bottom_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Percentage(50),
                Constraint::Fill(1),
            ])
            .areas(area);

        // Header line
        let header_text = format!(
            " Conflict: {} — {}/{} resolved",
            self.file.path, state.resolved_count, state.total_conflicts
        );
        let header_line = Line::from(header_text).style(Theme::diff_hunk_header());
        header_line.render(header_area, buf);

        // Split top half horizontally: OURS (left) | THEIRS (right)
        let [ours_area, theirs_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Fill(1)])
            .areas(top_area);

        // Get the current conflict's content
        let mut conflict_idx = 0;
        let mut ours_lines: Vec<String> = Vec::new();
        let mut theirs_lines: Vec<String> = Vec::new();

        for region in &self.file.regions {
            if let ConflictRegion::Conflict { ours, theirs } = region {
                if conflict_idx == state.current_conflict {
                    ours_lines = ours.clone();
                    theirs_lines = theirs.clone();
                    break;
                }
                conflict_idx += 1;
            }
        }

        // Render OURS panel
        let ours_text: Text = Text::from(
            ours_lines
                .iter()
                .map(|l| Line::from(l.as_str()).style(Theme::conflict_ours()))
                .collect::<Vec<_>>(),
        );
        let ours_block = Block::default()
            .title(" OURS ")
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .style(Theme::conflict_ours());
        Paragraph::new(ours_text)
            .block(ours_block)
            .render(ours_area, buf);

        // Render THEIRS panel
        let theirs_text: Text = Text::from(
            theirs_lines
                .iter()
                .map(|l| Line::from(l.as_str()).style(Theme::conflict_theirs()))
                .collect::<Vec<_>>(),
        );
        let theirs_block = Block::default()
            .title(" THEIRS ")
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .style(Theme::conflict_theirs());
        Paragraph::new(theirs_text)
            .block(theirs_block)
            .render(theirs_area, buf);

        // Render RESULT panel (editable)
        let result_text: Text = Text::from(
            state
                .result_lines
                .iter()
                .map(|l| Line::from(l.as_str()).style(Theme::conflict_result()))
                .collect::<Vec<_>>(),
        );
        let result_block = Block::default()
            .title(" RESULT ")
            .borders(Borders::ALL)
            .border_style(if state.editing {
                Theme::border_focused()
            } else {
                Theme::border()
            })
            .style(Theme::conflict_result());
        Paragraph::new(result_text)
            .block(result_block)
            .scroll((state.scroll_y, 0))
            .render(bottom_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitat_core::conflict::{ConflictFile, ConflictRegion};

    fn make_test_conflict() -> ConflictFile {
        ConflictFile {
            path: "test.rs".to_string(),
            regions: vec![
                ConflictRegion::Clean(vec!["line1".to_string()]),
                ConflictRegion::Conflict {
                    ours: vec!["ours_line".to_string()],
                    theirs: vec!["theirs_line".to_string()],
                },
                ConflictRegion::Clean(vec!["line3".to_string()]),
            ],
        }
    }

    #[test]
    fn test_initial_result_uses_ours() {
        let file = make_test_conflict();
        let state = ConflictEditorState::from_conflict_file(&file);
        assert_eq!(state.result_lines, vec!["line1", "ours_line", "line3"]);
        assert_eq!(state.total_conflicts, 1);
    }

    #[test]
    fn test_use_theirs() {
        let file = make_test_conflict();
        let mut state = ConflictEditorState::from_conflict_file(&file);
        state.use_theirs(&file);
        assert_eq!(state.result_lines, vec!["line1", "theirs_line", "line3"]);
    }

    #[test]
    fn test_use_ours() {
        let file = make_test_conflict();
        let mut state = ConflictEditorState::from_conflict_file(&file);
        state.use_theirs(&file);
        state.use_ours(&file);
        assert_eq!(state.result_lines, vec!["line1", "ours_line", "line3"]);
    }
}
