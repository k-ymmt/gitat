use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}
