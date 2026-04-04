use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn tab_active() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }
    pub fn tab_inactive() -> Style {
        Style::new().fg(Color::DarkGray)
    }
    pub fn file_staged() -> Style {
        Style::new().fg(Color::Green)
    }
    pub fn file_unstaged() -> Style {
        Style::new().fg(Color::Red)
    }
    pub fn file_untracked() -> Style {
        Style::new().fg(Color::Gray)
    }
    pub fn file_added() -> Style {
        Style::new().fg(Color::Green)
    }
    pub fn diff_added() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(28, 61, 28))
    }
    pub fn diff_removed() -> Style {
        Style::new().fg(Color::Red).bg(Color::Rgb(61, 28, 28))
    }
    pub fn diff_word_added() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(44, 107, 44))
    }
    pub fn diff_word_removed() -> Style {
        Style::new().fg(Color::Red).bg(Color::Rgb(107, 44, 44))
    }
    pub fn diff_context() -> Style {
        Style::new().fg(Color::Gray)
    }
    pub fn diff_line_number() -> Style {
        Style::new().fg(Color::DarkGray)
    }
    pub fn diff_hunk_header() -> Style {
        Style::new().fg(Color::Cyan)
    }
    pub fn status_bar() -> Style {
        Style::new().fg(Color::White).bg(Color::DarkGray)
    }
    pub fn help_key() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }
    pub fn conflict_ours() -> Style {
        Style::new()
            .fg(Color::Rgb(232, 168, 56))
            .bg(Color::Rgb(42, 34, 16))
    }
    pub fn conflict_theirs() -> Style {
        Style::new()
            .fg(Color::Rgb(56, 168, 232))
            .bg(Color::Rgb(16, 34, 42))
    }
    pub fn conflict_result() -> Style {
        Style::new().fg(Color::Green).bg(Color::Rgb(13, 26, 13))
    }
    pub fn selected() -> Style {
        Style::new()
            .bg(Color::Rgb(50, 50, 80))
            .add_modifier(Modifier::BOLD)
    }
    pub fn border() -> Style {
        Style::new().fg(Color::DarkGray)
    }
    pub fn border_focused() -> Style {
        Style::new().fg(Color::Cyan)
    }
    pub fn branch_current() -> Style {
        Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)
    }
    pub fn commit_hash() -> Style {
        Style::new().fg(Color::Yellow)
    }
    pub fn commit_ref() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }
    pub fn default_style() -> Style {
        Style::new()
    }

    const GRAPH_COLORS: [Color; 6] = [
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
    ];

    pub fn graph_color(index: usize) -> Style {
        Style::new().fg(Self::GRAPH_COLORS[index % Self::GRAPH_COLORS.len()])
    }
}
