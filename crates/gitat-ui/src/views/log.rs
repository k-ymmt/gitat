use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use gitat_core::commit_detail::FileChangeStatus;
use crate::app::{App, Mode, Panel};
use crate::theme::Theme;
use crate::widgets::side_by_side_diff::SideBySideDiff;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    if matches!(app.mode, Mode::CommitDetail) {
        render_commit_detail(f, app, area);
    } else {
        render_log_list(f, app, area);
    }
}

fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.log_entries.iter().map(|c| {
        let mut spans = vec![
            Span::styled(&c.short_hash, Theme::commit_hash()),
            Span::raw(" "),
        ];
        if !c.refs.is_empty() {
            spans.push(Span::styled(
                format!("({}) ", c.refs.join(", ")),
                Theme::commit_ref(),
            ));
        }
        spans.push(Span::raw(&c.message));
        ListItem::new(Line::from(spans))
    }).collect();

    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.log_list_state);
}

fn render_commit_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let commit = match &app.commit_detail_commit {
        Some(c) => c,
        None => return,
    };

    // Split: metadata (3 lines) + panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Metadata
    let meta_lines = vec![
        Line::from(vec![
            Span::styled(&commit.short_hash, Theme::commit_hash()),
            Span::raw("  "),
            Span::raw(&commit.author),
            Span::raw("  "),
            Span::styled(&commit.date, Theme::diff_context()),
        ]),
        Line::from(Span::raw(&commit.message)),
        Line::from(""),
    ];
    let meta = Paragraph::new(meta_lines)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Theme::border()));
    f.render_widget(meta, chunks[0]);

    // Two panels
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    render_file_list(f, app, panels[0]);
    render_commit_diff(f, app, panels[1]);
}

fn render_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.commit_detail_panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let items: Vec<ListItem> = app.commit_detail_files.iter().map(|entry| {
        let (code, style) = match entry.status {
            FileChangeStatus::Added => ("A", Theme::file_added()),
            FileChangeStatus::Modified => ("M", Theme::commit_hash()),
            FileChangeStatus::Deleted => ("D", Theme::file_unstaged()),
            FileChangeStatus::Renamed => ("R", Theme::commit_ref()),
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{code} "), style),
            Span::raw(&entry.path),
        ]))
    }).collect();

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.commit_detail_file_state);
}

fn render_commit_diff(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.commit_detail_panel == Panel::Right;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    if let Some(ref diff_files) = app.commit_detail_diff {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let widget = SideBySideDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.commit_detail_diff_state);
    } else {
        let block = Block::default()
            .title(" Diff ")
            .borders(Borders::ALL)
            .border_style(border_style);
        let placeholder = Paragraph::new("Select a file to view diff")
            .block(block)
            .style(Theme::diff_context());
        f.render_widget(placeholder, area);
    }
}
