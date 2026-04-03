use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.branches.iter().map(|b| {
        if b.is_current {
            ListItem::new(Line::from(vec![
                Span::styled("* ", Theme::branch_current()),
                Span::styled(&b.name, Theme::branch_current()),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::raw(&b.name),
            ]))
        }
    }).collect();

    let block = Block::default()
        .title(" Branches ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}
