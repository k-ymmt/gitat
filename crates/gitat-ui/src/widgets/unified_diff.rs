use gitat_core::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, StatefulWidget, Widget};
use similar::{ChangeTag, TextDiff};
use unicode_width::UnicodeWidthChar;

use crate::theme::Theme;

/// A paired row in the side-by-side display
struct DiffRow {
    left_line_no: Option<u32>,
    left_content: Option<String>,
    left_kind: DiffLineKind,
    right_line_no: Option<u32>,
    right_content: Option<String>,
    right_kind: DiffLineKind,
}

pub struct SideBySideDiffState {
    pub scroll_y: u16,
    pub scroll_x: u16,
    pub current_hunk: usize,
    hunk_offsets: Vec<u16>,
}

impl Default for SideBySideDiffState {
    fn default() -> Self {
        Self::new()
    }
}

impl SideBySideDiffState {
    pub fn new() -> Self {
        Self {
            scroll_y: 0,
            scroll_x: 0,
            current_hunk: 0,
            hunk_offsets: Vec::new(),
        }
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_y = self.scroll_y.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_y = self.scroll_y.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: u16) {
        self.scroll_x = self.scroll_x.saturating_add(amount);
    }

    pub fn scroll_left(&mut self, amount: u16) {
        self.scroll_x = self.scroll_x.saturating_sub(amount);
    }

    pub fn next_hunk(&mut self) {
        if self.current_hunk + 1 < self.hunk_offsets.len() {
            self.current_hunk += 1;
            self.scroll_y = self.hunk_offsets[self.current_hunk];
        }
    }

    pub fn prev_hunk(&mut self) {
        if self.current_hunk > 0 {
            self.current_hunk -= 1;
            self.scroll_y = self.hunk_offsets[self.current_hunk];
        }
    }
}

pub struct SideBySideDiff<'a> {
    diff_files: &'a [DiffFile],
    block: Option<Block<'a>>,
}

impl<'a> SideBySideDiff<'a> {
    pub fn new(diff_files: &'a [DiffFile]) -> Self {
        Self {
            diff_files,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

/// Takes a `DiffHunk` and returns paired rows for side-by-side display.
fn pair_lines(hunk: &DiffHunk) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let mut removed_buf: Vec<&DiffLine> = Vec::new();
    let mut added_buf: Vec<&DiffLine> = Vec::new();

    let flush = |rows: &mut Vec<DiffRow>,
                 removed_buf: &mut Vec<&DiffLine>,
                 added_buf: &mut Vec<&DiffLine>| {
        let max_len = removed_buf.len().max(added_buf.len());
        for i in 0..max_len {
            let left = removed_buf.get(i).copied();
            let right = added_buf.get(i).copied();
            rows.push(DiffRow {
                left_line_no: left.and_then(|l| l.old_line_no),
                left_content: left.map(|l| l.content.clone()),
                left_kind: if left.is_some() {
                    DiffLineKind::Removed
                } else {
                    DiffLineKind::Context
                },
                right_line_no: right.and_then(|l| l.new_line_no),
                right_content: right.map(|l| l.content.clone()),
                right_kind: if right.is_some() {
                    DiffLineKind::Added
                } else {
                    DiffLineKind::Context
                },
            });
        }
        removed_buf.clear();
        added_buf.clear();
    };

    for line in &hunk.lines {
        match line.kind {
            DiffLineKind::Context => {
                flush(&mut rows, &mut removed_buf, &mut added_buf);
                rows.push(DiffRow {
                    left_line_no: line.old_line_no,
                    left_content: Some(line.content.clone()),
                    left_kind: DiffLineKind::Context,
                    right_line_no: line.new_line_no,
                    right_content: Some(line.content.clone()),
                    right_kind: DiffLineKind::Context,
                });
            }
            DiffLineKind::Removed => {
                removed_buf.push(line);
            }
            DiffLineKind::Added => {
                added_buf.push(line);
            }
        }
    }

    // Flush remaining buffers
    flush(&mut rows, &mut removed_buf, &mut added_buf);

    rows
}

/// Represents a styled text segment for word-level diff rendering.
struct StyledSegment {
    text: String,
    style: Style,
}

/// Renders content with word-level diff highlighting.
///
/// When both `old_content` and `new_content` are provided (a changed line that has a pair),
/// we use `similar::TextDiff::from_words` to compute word-level changes.
///
/// For left side (is_left=true): highlight Delete words with word_removed style
/// For right side (is_left=false): highlight Insert words with word_added style
fn render_content_with_word_diff(
    content: &str,
    paired_content: Option<&str>,
    kind: &DiffLineKind,
    is_left: bool,
) -> Vec<StyledSegment> {
    let base_style = match kind {
        DiffLineKind::Added => Theme::diff_added(),
        DiffLineKind::Removed => Theme::diff_removed(),
        DiffLineKind::Context => Theme::diff_context(),
    };

    // If there's no paired content (no word-level diff possible), return simple styled content
    let paired = match paired_content {
        Some(p) => p,
        None => {
            return vec![StyledSegment {
                text: content.to_string(),
                style: base_style,
            }];
        }
    };

    // Only do word-level diff for changed lines
    if matches!(kind, DiffLineKind::Context) {
        return vec![StyledSegment {
            text: content.to_string(),
            style: base_style,
        }];
    }

    let (old, new) = if is_left {
        (content, paired)
    } else {
        (paired, content)
    };

    let text_diff = TextDiff::from_words(old, new);
    let mut segments = Vec::new();

    for change in text_diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                if is_left {
                    // On the left side, Equal text appears in old
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: base_style,
                    });
                } else {
                    // On the right side, Equal text appears in new
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: base_style,
                    });
                }
            }
            ChangeTag::Delete => {
                if is_left {
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: Theme::diff_word_removed(),
                    });
                }
                // Skip Delete changes on the right side
            }
            ChangeTag::Insert => {
                if !is_left {
                    segments.push(StyledSegment {
                        text: change.value().to_string(),
                        style: Theme::diff_word_added(),
                    });
                }
                // Skip Insert changes on the left side
            }
        }
    }

    segments
}

/// Write styled segments into the buffer at the given position, handling horizontal scroll.
fn write_segments(
    buf: &mut Buffer,
    segments: &[StyledSegment],
    x_start: u16,
    y: u16,
    max_width: u16,
    scroll_x: u16,
    bg_style: Style,
) {
    // First, fill the entire area with the background style
    for dx in 0..max_width {
        let px = x_start + dx;
        if let Some(cell) = buf.cell_mut((px, y)) {
            cell.set_char(' ');
            cell.set_style(bg_style);
        }
    }

    // Flatten all segments into a list of (char, style) to handle scroll_x properly
    let mut chars: Vec<(char, Style)> = Vec::new();
    for seg in segments {
        for ch in seg.text.chars() {
            chars.push((ch, seg.style));
        }
    }

    // Apply horizontal scroll: skip characters whose total display width >= scroll_x
    let mut skipped_width: u16 = 0;
    let mut skip_count = 0;
    for &(ch, _) in &chars {
        let w = ch.width().unwrap_or(0) as u16;
        if skipped_width + w > scroll_x {
            break;
        }
        skipped_width += w;
        skip_count += 1;
    }

    let visible_chars = chars.into_iter().skip(skip_count);
    let mut col: u16 = 0;
    for (ch, style) in visible_chars {
        let w = ch.width().unwrap_or(0) as u16;
        if col + w > max_width {
            break;
        }
        let px = x_start + col;
        if let Some(cell) = buf.cell_mut((px, y)) {
            cell.set_char(ch);
            cell.set_style(style);
        }
        col += w;
    }
}

impl StatefulWidget for SideBySideDiff<'_> {
    type State = SideBySideDiffState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Handle block
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Early return if area too small
        if inner.width < 10 || inner.height < 2 {
            return;
        }

        // Build all DiffRows from all files/hunks, recording hunk_offsets
        let mut all_rows: Vec<DiffRow> = Vec::new();
        let mut hunk_offsets: Vec<u16> = Vec::new();

        for file in self.diff_files {
            for hunk in &file.hunks {
                hunk_offsets.push(all_rows.len() as u16);
                let rows = pair_lines(hunk);
                all_rows.extend(rows);
            }
        }

        state.hunk_offsets = hunk_offsets;

        // Calculate layout
        let line_no_width: u16 = 4;
        let separator_width: u16 = 1;
        // Total width for one side = line_no_width + 1(space) + content_width
        // Layout: [left_line_no(4)] [left_content] [sep(1)] [right_line_no(4)] [right_content]
        let half_width = inner.width / 2;
        let left_content_width = half_width.saturating_sub(line_no_width + 1); // +1 for space after line no
        let right_start_x = inner.x + half_width + separator_width;
        let right_content_width = inner
            .width
            .saturating_sub(half_width + separator_width + line_no_width + 1);

        // For each visible row (based on scroll_y)
        for row_idx in 0..inner.height {
            let data_idx = state.scroll_y as usize + row_idx as usize;
            let y = inner.y + row_idx;

            if data_idx >= all_rows.len() {
                break;
            }

            let row = &all_rows[data_idx];

            // --- Left side ---
            // Line number
            let left_line_no_str = match row.left_line_no {
                Some(n) => format!("{:>width$}", n, width = line_no_width as usize),
                None => " ".repeat(line_no_width as usize),
            };
            buf.set_string(inner.x, y, &left_line_no_str, Theme::diff_line_number());

            // Left content
            let left_content_x = inner.x + line_no_width + 1; // 1 space after line number
            let left_bg_style = match row.left_kind {
                DiffLineKind::Added => Theme::diff_added(),
                DiffLineKind::Removed => Theme::diff_removed(),
                DiffLineKind::Context => Theme::diff_context(),
            };

            if let Some(ref content) = row.left_content {
                // Determine if we have a paired change for word-level diff
                let paired = if matches!(row.left_kind, DiffLineKind::Removed)
                    && matches!(row.right_kind, DiffLineKind::Added)
                {
                    row.right_content.as_deref()
                } else {
                    None
                };

                let segments =
                    render_content_with_word_diff(content, paired, &row.left_kind, true);
                write_segments(
                    buf,
                    &segments,
                    left_content_x,
                    y,
                    left_content_width,
                    state.scroll_x,
                    left_bg_style,
                );
            } else {
                // Empty side - fill with background
                write_segments(
                    buf,
                    &[],
                    left_content_x,
                    y,
                    left_content_width,
                    0,
                    Theme::diff_context(),
                );
            }

            // --- Separator ---
            let sep_x = inner.x + half_width;
            buf.set_string(sep_x, y, "\u{2502}", Theme::diff_line_number());

            // --- Right side ---
            // Line number
            let right_line_no_str = match row.right_line_no {
                Some(n) => format!("{:>width$}", n, width = line_no_width as usize),
                None => " ".repeat(line_no_width as usize),
            };
            buf.set_string(right_start_x, y, &right_line_no_str, Theme::diff_line_number());

            // Right content
            let right_content_x = right_start_x + line_no_width + 1;
            let right_bg_style = match row.right_kind {
                DiffLineKind::Added => Theme::diff_added(),
                DiffLineKind::Removed => Theme::diff_removed(),
                DiffLineKind::Context => Theme::diff_context(),
            };

            if let Some(ref content) = row.right_content {
                let paired = if matches!(row.right_kind, DiffLineKind::Added)
                    && matches!(row.left_kind, DiffLineKind::Removed)
                {
                    row.left_content.as_deref()
                } else {
                    None
                };

                let segments =
                    render_content_with_word_diff(content, paired, &row.right_kind, false);
                write_segments(
                    buf,
                    &segments,
                    right_content_x,
                    y,
                    right_content_width,
                    state.scroll_x,
                    right_bg_style,
                );
            } else {
                write_segments(
                    buf,
                    &[],
                    right_content_x,
                    y,
                    right_content_width,
                    0,
                    Theme::diff_context(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitat_core::diff::{DiffFile, DiffHunk, DiffLine, DiffLineKind};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    fn buffer_to_string(buf: &Buffer) -> String {
        let mut s = String::new();
        for y in buf.area.y..buf.area.y + buf.area.height {
            for x in buf.area.x..buf.area.x + buf.area.width {
                let cell = buf.cell((x, y)).unwrap();
                s.push_str(cell.symbol());
            }
            // trim trailing spaces per line for cleaner snapshots
            let trimmed = s.trim_end_matches(' ');
            s.truncate(trimmed.len());
            s.push('\n');
        }
        s
    }

    #[test]
    fn test_pair_lines_context() {
        let hunk = DiffHunk {
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines: vec![DiffLine {
                kind: DiffLineKind::Context,
                content: "hello".to_string(),
                old_line_no: Some(1),
                new_line_no: Some(1),
            }],
        };
        let rows = pair_lines(&hunk);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].left_content.as_deref(), Some("hello"));
        assert_eq!(rows[0].right_content.as_deref(), Some("hello"));
    }

    #[test]
    fn test_pair_lines_change() {
        let hunk = DiffHunk {
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines: vec![
                DiffLine {
                    kind: DiffLineKind::Removed,
                    content: "old".to_string(),
                    old_line_no: Some(1),
                    new_line_no: None,
                },
                DiffLine {
                    kind: DiffLineKind::Added,
                    content: "new".to_string(),
                    old_line_no: None,
                    new_line_no: Some(1),
                },
            ],
        };
        let rows = pair_lines(&hunk);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].left_content.as_deref(), Some("old"));
        assert_eq!(rows[0].right_content.as_deref(), Some("new"));
    }

    #[test]
    fn test_render_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "test.rs".to_string(),
            new_path: "test.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "    old_code();".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    new_code();".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "}".to_string(),
                        old_line_no: Some(3),
                        new_line_no: Some(3),
                    },
                ],
            }],
        }];
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_hunk_navigation() {
        let mut state = SideBySideDiffState::new();
        state.hunk_offsets = vec![0, 10, 25];
        state.next_hunk();
        assert_eq!(state.current_hunk, 1);
        assert_eq!(state.scroll_y, 10);
        state.next_hunk();
        assert_eq!(state.current_hunk, 2);
        assert_eq!(state.scroll_y, 25);
        state.next_hunk(); // no more
        assert_eq!(state.current_hunk, 2);
        state.prev_hunk();
        assert_eq!(state.current_hunk, 1);
        assert_eq!(state.scroll_y, 10);
    }

    fn make_simple_diff() -> Vec<DiffFile> {
        vec![DiffFile {
            old_path: "test.rs".to_string(),
            new_path: "test.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "fn main() {".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "    old_code();".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "    new_code();".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "}".to_string(),
                        old_line_no: Some(3),
                        new_line_no: Some(3),
                    },
                ],
            }],
        }]
    }

    #[test]
    fn snapshot_render_context_and_changes() {
        let backend = TestBackend::new(60, 6);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = make_simple_diff();
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_additions_only() {
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "/dev/null".to_string(),
            new_path: "new.rs".to_string(),
            hunks: vec![DiffHunk {
                old_start: 0,
                old_count: 0,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "fn hello() {}".to_string(),
                        old_line_no: None,
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "fn world() {}".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                ],
            }],
        }];
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_deletions_only() {
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "old.rs".to_string(),
            new_path: "/dev/null".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 2,
                new_start: 0,
                new_count: 0,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "fn removed1() {}".to_string(),
                        old_line_no: Some(1),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "fn removed2() {}".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                ],
            }],
        }];
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_with_horizontal_scroll() {
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = make_simple_diff();
        let mut state = SideBySideDiffState::new();
        state.scroll_x = 4; // scroll right by 4 chars
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_narrow_terminal() {
        let backend = TestBackend::new(30, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = make_simple_diff();
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_multiple_hunks() {
        let backend = TestBackend::new(60, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "lib.rs".to_string(),
            new_path: "lib.rs".to_string(),
            hunks: vec![
                DiffHunk {
                    old_start: 1,
                    old_count: 2,
                    new_start: 1,
                    new_count: 2,
                    lines: vec![
                        DiffLine {
                            kind: DiffLineKind::Context,
                            content: "fn first() {".to_string(),
                            old_line_no: Some(1),
                            new_line_no: Some(1),
                        },
                        DiffLine {
                            kind: DiffLineKind::Removed,
                            content: "    old1();".to_string(),
                            old_line_no: Some(2),
                            new_line_no: None,
                        },
                        DiffLine {
                            kind: DiffLineKind::Added,
                            content: "    new1();".to_string(),
                            old_line_no: None,
                            new_line_no: Some(2),
                        },
                    ],
                },
                DiffHunk {
                    old_start: 10,
                    old_count: 2,
                    new_start: 10,
                    new_count: 2,
                    lines: vec![
                        DiffLine {
                            kind: DiffLineKind::Context,
                            content: "fn second() {".to_string(),
                            old_line_no: Some(10),
                            new_line_no: Some(10),
                        },
                        DiffLine {
                            kind: DiffLineKind::Removed,
                            content: "    old2();".to_string(),
                            old_line_no: Some(11),
                            new_line_no: None,
                        },
                        DiffLine {
                            kind: DiffLineKind::Added,
                            content: "    new2();".to_string(),
                            old_line_no: None,
                            new_line_no: Some(11),
                        },
                    ],
                },
            ],
        }];
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_cjk_characters() {
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "hello.txt".to_string(),
            new_path: "hello.txt".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 2,
                new_start: 1,
                new_count: 2,
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        content: "こんにちは世界".to_string(),
                        old_line_no: Some(1),
                        new_line_no: Some(1),
                    },
                    DiffLine {
                        kind: DiffLineKind::Removed,
                        content: "古いコード".to_string(),
                        old_line_no: Some(2),
                        new_line_no: None,
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        content: "新しいコード".to_string(),
                        old_line_no: None,
                        new_line_no: Some(2),
                    },
                ],
            }],
        }];
        let mut state = SideBySideDiffState::new();
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }

    #[test]
    fn snapshot_render_cjk_with_horizontal_scroll() {
        let backend = TestBackend::new(60, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        let diff = vec![DiffFile {
            old_path: "hello.txt".to_string(),
            new_path: "hello.txt".to_string(),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                lines: vec![DiffLine {
                    kind: DiffLineKind::Context,
                    content: "あいうえおかきくけこ".to_string(),
                    old_line_no: Some(1),
                    new_line_no: Some(1),
                }],
            }],
        }];
        let mut state = SideBySideDiffState::new();
        state.scroll_x = 4; // skip 2 CJK chars (4 display columns)
        terminal
            .draw(|f| {
                let widget = SideBySideDiff::new(&diff);
                f.render_stateful_widget(widget, f.area(), &mut state);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        insta::assert_snapshot!(buffer_to_string(&buf));
    }
}
