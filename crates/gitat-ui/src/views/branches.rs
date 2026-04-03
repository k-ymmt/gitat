use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use crate::app::App;
use crate::theme::Theme;

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.branches.iter().map(|b| {
        let prefix = if b.is_current { "* " } else { "  " };
        let name_style = if b.is_current { Theme::branch_current() } else { Theme::default_style() };
        ListItem::new(Line::from(vec![
            Span::styled(prefix, name_style),
            Span::styled(&b.name, name_style),
            Span::raw(" "),
            Span::styled(&b.short_hash, Theme::commit_hash()),
            Span::raw(" "),
            Span::raw(&b.last_commit),
        ]))
    }).collect();

    let block = Block::default()
        .title(" Branches ")
        .borders(Borders::ALL)
        .border_style(Theme::border());

    let list = List::new(items)
        .block(block)
        .highlight_style(Theme::selected());

    f.render_stateful_widget(list, area, &mut app.branches_list_state);
}
