use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use gitat_core::status::FileStatus;
use crate::app::{App, Panel};
use crate::theme::Theme;
use crate::widgets::side_by_side_diff::SideBySideDiff;

fn file_status_code(status: &FileStatus) -> &'static str {
    match status {
        FileStatus::Modified => "M",
        FileStatus::Added => "A",
        FileStatus::Deleted => "D",
        FileStatus::Renamed => "R",
        FileStatus::Copied => "C",
        FileStatus::Untracked => "?",
        FileStatus::Unmodified => " ",
    }
}

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    render_file_list(f, app, chunks[0]);
    render_diff_detail(f, app, chunks[1]);
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let mut items: Vec<ListItem> = Vec::new();

    // Staged section
    let staged: Vec<_> = app.status.iter().filter(|e| {
        e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
    }).collect();

    if !staged.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Staged",
            Theme::file_staged(),
        ))));
        for entry in &staged {
            let code = file_status_code(&entry.index_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_staged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    // Modified (unstaged) section
    let modified: Vec<_> = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Unmodified
            && e.worktree_status != FileStatus::Unmodified
            && e.worktree_status != FileStatus::Untracked
    }).collect();

    if !modified.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Modified",
            Theme::file_unstaged(),
        ))));
        for entry in &modified {
            let code = file_status_code(&entry.worktree_status);
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{code} "), Theme::file_unstaged()),
                Span::raw(&entry.path),
            ])));
        }
    }

    // Untracked section
    let untracked: Vec<_> = app.status.iter().filter(|e| {
        e.index_status == FileStatus::Untracked
    }).collect();

    if !untracked.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Untracked",
            Theme::file_untracked(),
        ))));
        for entry in &untracked {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("? ", Theme::file_untracked()),
                Span::raw(&entry.path),
            ])));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "Nothing to show",
            Theme::file_untracked(),
        ))));
    }

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}

fn render_diff_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.panel == Panel::Right;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.current_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let widget = SideBySideDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.diff_state);
    } else {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let placeholder = Paragraph::new("Select a file and press Enter to view diff")
            .block(block)
            .style(Theme::diff_context());

        f.render_widget(placeholder, area);
    }
}
