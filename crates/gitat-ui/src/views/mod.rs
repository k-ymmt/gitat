pub mod branches;
pub mod commit;
pub mod log;

use ratatui::Frame;
use ratatui::layout::Rect;
use crate::app::{App, Tab};

pub fn render_tab(f: &mut Frame, app: &mut App, area: Rect) {
    match app.tab {
        Tab::Log => log::render(f, app, area),
        Tab::Branches => branches::render(f, app, area),
        Tab::Stash => {} // Placeholder
    }
}
