use crate::app::{App, Mode};
use crate::theme::Theme;
use crate::util::centered_rect;
use ratatui::Frame;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_commit_popup(f: &mut Frame, app: &App) {
    let Mode::Commit { message } = &app.mode else {
        return;
    };

    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Commit Message ")
        .borders(Borders::ALL)
        .border_style(Theme::border_focused());

    let display = format!("{message}_");
    let paragraph = Paragraph::new(display).block(block);
    f.render_widget(paragraph, area);
}
