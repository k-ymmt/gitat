use crate::app::{App, Mode, Panel};
use crate::theme::Theme;
use crate::widgets::unified_diff::UnifiedDiff;
use gitat_core::commit_detail::FileChangeStatus;
use gitat_core::graph;
use gitat_core::status::FileStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    match app.mode {
        Mode::CommitDetail => render_commit_detail(f, app, area),
        Mode::UncommittedDetail => render_uncommitted_detail(f, app, area),
        _ => render_log_list(f, app, area),
    }
}

fn render_log_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_searching = matches!(app.mode, Mode::Search { .. });

    let outer_chunks = if is_searching {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    let main_area = outer_chunks[0];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_area);

    render_log_list_items(f, app, chunks[0]);

    match app.log_list_state.selected() {
        Some(0) if app.filtered_log_indices.is_none() => {
            render_uncommitted_preview(f, app, chunks[1]);
        }
        Some(_) => render_commit_preview(f, app, chunks[1]),
        None => {}
    }

    // Search bar
    if is_searching && let Mode::Search { ref query } = app.mode {
        let search_text = format!("/{query}_");
        let search_bar = Paragraph::new(search_text).style(Theme::status_bar());
        f.render_widget(search_bar, outer_chunks[1]);
    }
}

fn render_log_list_items(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = if let Some(ref indices) = app.filtered_log_indices {
        // Filtered mode: show only matching commits, no graph, no uncommitted row
        indices
            .iter()
            .map(|&i| {
                let c = &app.log_entries[i];
                let mut spans: Vec<Span> = Vec::new();
                spans.push(Span::styled(&c.short_hash, Theme::commit_hash()));
                spans.push(Span::raw(" "));
                if !c.refs.is_empty() {
                    spans.push(Span::styled(
                        format!("({}) ", c.refs.join(", ")),
                        Theme::commit_ref(),
                    ));
                }
                spans.push(Span::raw(&c.message));
                ListItem::new(Line::from(spans))
            })
            .collect()
    } else {
        // Normal mode: full rendering with graph and uncommitted row
        let mut items: Vec<ListItem> = Vec::new();
        let graph_rows = graph::build_graph(&app.log_entries);

        // Uncommitted changes item (always at index 0)
        let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
        let unstaged_count = app
            .status
            .iter()
            .filter(|e| !e.is_staged() && e.worktree_status != FileStatus::Untracked)
            .count();
        let untracked_count = app
            .status
            .iter()
            .filter(|e| e.index_status == FileStatus::Untracked)
            .count();
        let total_changes = staged_count + unstaged_count + untracked_count;

        let mut uncommitted_spans: Vec<Span> = Vec::new();
        if let Some(first_row) = graph_rows.first() {
            if total_changes > 0 {
                uncommitted_spans.push(Span::styled("* ", Theme::graph_color(0)));
                for cell in first_row.cells.iter().skip(1) {
                    if cell.symbol != ' ' {
                        uncommitted_spans
                            .push(Span::styled("| ", Theme::graph_color(cell.color_index)));
                    } else {
                        uncommitted_spans.push(Span::raw("  "));
                    }
                }
            } else {
                for cell in &first_row.cells {
                    if cell.symbol != ' ' {
                        uncommitted_spans
                            .push(Span::styled("| ", Theme::graph_color(cell.color_index)));
                    } else {
                        uncommitted_spans.push(Span::raw("  "));
                    }
                }
            }
        } else if total_changes > 0 {
            uncommitted_spans.push(Span::styled("* ", Theme::graph_color(0)));
        }

        if total_changes > 0 {
            uncommitted_spans.extend([
                Span::styled("● ", Theme::border_focused()),
                Span::styled("Uncommitted Changes", Theme::border_focused()),
                Span::styled(
                    format!(
                        " — {} staged, {} unstaged",
                        staged_count,
                        unstaged_count + untracked_count
                    ),
                    Theme::diff_context(),
                ),
            ]);
        } else {
            uncommitted_spans.extend([
                Span::styled("● ", Theme::border_focused()),
                Span::styled("Uncommitted Changes", Theme::border_focused()),
            ]);
        }
        items.push(ListItem::new(Line::from(uncommitted_spans)));

        for (i, c) in app.log_entries.iter().enumerate() {
            let mut spans: Vec<Span> = Vec::new();
            if let Some(row) = graph_rows.get(i) {
                for cell in &row.cells {
                    let s = format!("{} ", cell.symbol);
                    spans.push(Span::styled(s, Theme::graph_color(cell.color_index)));
                }
            }
            spans.push(Span::styled(&c.short_hash, Theme::commit_hash()));
            spans.push(Span::raw(" "));
            if !c.refs.is_empty() {
                spans.push(Span::styled(
                    format!("({}) ", c.refs.join(", ")),
                    Theme::commit_ref(),
                ));
            }
            spans.push(Span::raw(&c.message));
            items.push(ListItem::new(Line::from(spans)));
        }

        items
    };

    let title = if app.filtered_log_indices.is_some() {
        " Log (filtered) "
    } else {
        " Log "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.log_list_state);
}

fn render_commit_preview(f: &mut Frame, app: &mut App, area: Rect) {
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
    let meta = Paragraph::new(meta_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border()),
    );
    f.render_widget(meta, chunks[0]);

    // File list (full width)
    let items: Vec<ListItem> = app
        .commit_detail_files
        .iter()
        .map(|entry| {
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
        })
        .collect();

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
}

fn render_uncommitted_preview(f: &mut Frame, app: &mut App, area: Rect) {
    // Split: header (1 line) + panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header
    let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
    let unstaged_count = app
        .status
        .iter()
        .filter(|e| !e.is_staged() && e.worktree_status != FileStatus::Untracked)
        .count();
    let untracked_count = app
        .status
        .iter()
        .filter(|e| e.index_status == FileStatus::Untracked)
        .count();
    let total_changes = staged_count + unstaged_count + untracked_count;

    let header_text = if total_changes > 0 {
        format!(
            "Uncommitted Changes — {} staged, {} unstaged",
            staged_count,
            unstaged_count + untracked_count
        )
    } else {
        "Uncommitted Changes".to_string()
    };
    let header = Paragraph::new(Line::from(Span::styled(
        header_text,
        Theme::border_focused(),
    )))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border()),
    );
    f.render_widget(header, chunks[0]);

    // File list (full width)
    let mut items: Vec<ListItem> = Vec::new();

    // Staged section
    let staged: Vec<_> = app
        .status
        .iter()
        .filter(|e| {
            e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
        })
        .collect();

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

    // Modified (unstaged) section — includes files that are also staged (e.g. MM)
    let modified: Vec<_> = app
        .status
        .iter()
        .filter(|e| {
            e.worktree_status != FileStatus::Unmodified
                && e.worktree_status != FileStatus::Untracked
        })
        .collect();

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
    let untracked: Vec<_> = app
        .status
        .iter()
        .filter(|e| e.index_status == FileStatus::Untracked)
        .collect();

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
            "No uncommitted changes",
            Theme::file_untracked(),
        ))));
    }

    let block = Block::default()
        .title(" Files ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, chunks[1]);
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
    let meta = Paragraph::new(meta_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border()),
    );
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

    let items: Vec<ListItem> = app
        .commit_detail_files
        .iter()
        .map(|entry| {
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
        })
        .collect();

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
        let widget = UnifiedDiff::new(diff_files).block(block);
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

fn render_uncommitted_detail(f: &mut Frame, app: &mut App, area: Rect) {
    // Split: header (1 line) + panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header
    let staged_count = app.status.iter().filter(|e| e.is_staged()).count();
    let unstaged_count = app
        .status
        .iter()
        .filter(|e| !e.is_staged() && e.worktree_status != FileStatus::Untracked)
        .count();
    let untracked_count = app
        .status
        .iter()
        .filter(|e| e.index_status == FileStatus::Untracked)
        .count();
    let total_changes = staged_count + unstaged_count + untracked_count;

    let header_text = if total_changes > 0 {
        format!(
            "Uncommitted Changes — {} staged, {} unstaged",
            staged_count,
            unstaged_count + untracked_count
        )
    } else {
        "Uncommitted Changes".to_string()
    };
    let header = Paragraph::new(Line::from(Span::styled(
        header_text,
        Theme::border_focused(),
    )))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Theme::border()),
    );
    f.render_widget(header, chunks[0]);

    // Two panels
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    render_uncommitted_file_list(f, app, panels[0]);
    render_uncommitted_diff(f, app, panels[1]);
}

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

fn render_uncommitted_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.panel == Panel::Left;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let mut items: Vec<ListItem> = Vec::new();

    // Staged section
    let staged: Vec<_> = app
        .status
        .iter()
        .filter(|e| {
            e.index_status != FileStatus::Unmodified && e.index_status != FileStatus::Untracked
        })
        .collect();

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

    // Modified (unstaged) section — includes files that are also staged (e.g. MM)
    let modified: Vec<_> = app
        .status
        .iter()
        .filter(|e| {
            e.worktree_status != FileStatus::Unmodified
                && e.worktree_status != FileStatus::Untracked
        })
        .collect();

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
    let untracked: Vec<_> = app
        .status
        .iter()
        .filter(|e| e.index_status == FileStatus::Untracked)
        .collect();

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
            "No uncommitted changes",
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

    f.render_stateful_widget(list, area, &mut app.uncommitted_list_state);
}

fn render_uncommitted_diff(f: &mut Frame, app: &mut App, area: Rect) {
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
        let widget = UnifiedDiff::new(diff_files).block(block);
        f.render_stateful_widget(widget, area, &mut app.diff_state);
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
