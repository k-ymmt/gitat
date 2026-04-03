---
name: ratatui
description: >
  Comprehensive reference for building TUI applications with ratatui 0.30.x.
  Use when writing ratatui code: creating widgets, layouts, event handling,
  styling, or testing. Triggers: ratatui, TUI, terminal UI, Widget, Layout,
  Constraint, crossterm, TestBackend.
---

# ratatui 0.30.x Reference

Target version: **ratatui 0.30.0** / Rust edition 2024.

---

## Quick Start

### Dependencies

```sh
cargo add ratatui crossterm
```

Crossterm is re-exported at `ratatui::crossterm`, so you can depend on ratatui
alone. Adding crossterm directly is the more common convention.

### Minimal App (ratatui::run)

`ratatui::run` (new in 0.30) handles terminal init/restore and catches panics.

```rust
use std::io;
use crossterm::event::{self, KeyEventKind};

fn main() -> io::Result<()> {
    ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| {
                frame.render_widget("Hello, ratatui!", frame.area());
            })?;
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    break Ok(());
                }
            }
        }
    })
}
```

### App Struct Pattern

For non-trivial applications, separate rendering and event handling:

```rust
use std::io;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

struct App { should_quit: bool }

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
    fn render(&self, frame: &mut Frame) {
        frame.render_widget("Hello from App!", frame.area());
    }
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App { should_quit: false }.run(terminal))
}
```

---

## Core Concepts

### Immediate Mode Rendering

Every frame, redraw the entire UI. ratatui diffs buffers internally and only
sends changes to the terminal.

1. Call `terminal.draw(|frame| { ... })`.
2. Split `frame.area()` into regions with `Layout`.
3. Render widgets into each region with `frame.render_widget(widget, area)`.
4. After the closure returns, ratatui flushes the diff.

### Terminal

```rust
pub type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;
```

- `ratatui::run(f)` -- init, run closure, restore. Catches panics.
- `ratatui::init()` -> `DefaultTerminal` -- manual init.
- `ratatui::restore()` -- manual restore (call in drop or panic hook).
- `terminal.draw(|frame| { ... })` -- render one frame.

### Frame

Provided inside `draw()`. Never construct it yourself.

```rust
frame.area() -> Rect                           // full terminal area
frame.render_widget(widget, area)              // render a Widget
frame.render_stateful_widget(widget, area, &mut state) // render with state
frame.buffer_mut() -> &mut Buffer              // direct buffer access
```

### Rect

A rectangle: `x`, `y`, `width`, `height` (all `u16`). Centering helpers (new in 0.30):

```rust
let popup = area.centered(Constraint::Length(60), Constraint::Length(20));
```

### Widget Traits

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

All built-in widgets implement `Widget`. `&str`, `String`, `Span`, `Line`,
and `Text` also implement `Widget`, so you can render plain strings directly.
Use `StatefulWidget` for widgets needing mutable state (List, Table, Scrollbar).

---

## Layout System

### Splitting Areas

```rust
use ratatui::layout::{Layout, Constraint, Flex};

// Vertical split with const-generic areas (preferred)
let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Fill(1),
    Constraint::Length(1),
]).areas(area);

// Horizontal split
let [left, right] = Layout::horizontal([
    Constraint::Percentage(30),
    Constraint::Fill(1),
]).areas(area);

// Dynamic split (N not known at compile time)
let chunks: Rc<[Rect]> = Layout::vertical(constraints).split(area);
```

### Constraint Types

| Constraint | Meaning |
|---|---|
| `Length(n)` | Exactly n cells |
| `Min(n)` | At least n cells |
| `Max(n)` | At most n cells |
| `Percentage(n)` | n% of available space |
| `Ratio(a, b)` | a/b of available space |
| `Fill(weight)` | Fill remaining space (weighted) |

### Flex and Modifiers

```rust
Layout::horizontal(constraints)
    .flex(Flex::Center)   // Legacy, Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly
    .spacing(1)           // gap between areas
    .margin(1)            // outer margin
```

Note: In 0.30 `SpaceAround` follows CSS flexbox semantics; `SpaceEvenly` added.

### Layout Macros

```rust
use ratatui::macros::{constraints, vertical, horizontal};

let c = constraints![==50, ==30%, >=3, <=1, ==1/2, *=1];
// ==N -> Length  ==N% -> Percentage  >=N -> Min  <=N -> Max  ==A/B -> Ratio  *=N -> Fill

let [top, bottom] = vertical![==1, *=1].areas(area);
let [left, right] = horizontal![==20, *=1].areas(area);
```

---

## Common Widgets Overview

### Block

Universal container. Most widgets accept `.block(block)`.

```rust
use ratatui::widgets::{Block, Padding};

let block = Block::bordered()
    .title_top(" Title ")
    .title_bottom(" Status ")
    .padding(Padding::horizontal(1));
```

In 0.30, `block::Title` was removed. Use `Into<Line>` directly with
`title_top()` / `title_bottom()`.

### Paragraph

```rust
let paragraph = Paragraph::new(vec![
    Line::from(vec![Span::raw("Hello "), "world".bold().red()]),
])
.block(Block::bordered().title_top(" Output "))
.wrap(ratatui::widgets::Wrap { trim: true })
.scroll((scroll_offset, 0));
```

### List (Stateful)

```rust
let list = List::new(["item 1", "item 2", "item 3"])
    .block(Block::bordered().title_top(" Files "))
    .highlight_style(Style::new().reversed())
    .highlight_symbol("> ");

let mut state = ListState::default().with_selected(Some(0));
frame.render_stateful_widget(list, area, &mut state);
```

### Table (Stateful)

```rust
let rows = vec![
    Row::new(vec!["abc123", "feat: add widget", "2h ago"]),
    Row::new(vec!["def456", "fix: layout bug", "5h ago"]),
];
let table = Table::new(rows, [
    Constraint::Length(8), Constraint::Fill(1), Constraint::Length(12),
])
.header(Row::new(vec!["Hash", "Message", "Date"]).bold())
.block(Block::bordered().title_top(" Commits "))
.highlight_style(Style::new().reversed());

let mut state = TableState::default().with_selected(Some(0));
frame.render_stateful_widget(table, area, &mut state);
```

### Tabs

```rust
let tabs = Tabs::new(vec!["Status", "Log", "Diff"])
    .block(Block::bordered())
    .select(current_tab)
    .highlight_style(Style::new().bold().underlined());
frame.render_widget(tabs, area);
```

### Gauge / LineGauge

```rust
let gauge = Gauge::default()
    .block(Block::bordered().title_top(" Progress "))
    .gauge_style(Style::new().cyan())
    .percent(42);
let line_gauge = LineGauge::default().gauge_style(Style::new().green()).ratio(0.42);
```

### Scrollbar (Stateful)

```rust
let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
let mut sb_state = ScrollbarState::new(total_items).position(current_pos);
frame.render_stateful_widget(scrollbar, area, &mut sb_state);
```

### Widget Selection Guide

| Need | Widget |
|---|---|
| Multiline text / wrapping | `Paragraph` |
| Selectable item list | `List` + `ListState` |
| Columnar data with selection | `Table` + `TableState` |
| Tab / mode switching | `Tabs` |
| Progress indicator | `Gauge` or `LineGauge` |
| Scroll position indicator | `Scrollbar` + `ScrollbarState` |
| Data visualization | `Chart`, `BarChart`, `Sparkline` |
| Custom graphics / shapes | `Canvas` |
| Calendar | `calendar::Monthly` |

For detailed widget API, see `references/widgets.md`.

---

## Event Handling

ratatui does not handle events. Use crossterm directly:

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
```

### Blocking Read

See the App struct pattern in Quick Start for the full loop. Key matching:

```rust
fn handle_key(&mut self, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => self.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
        KeyCode::Enter => self.select(),
        KeyCode::Esc => self.back(),
        _ => {}
    }
}
```

### Non-blocking (Poll + Read)

Use `event::poll()` for timed updates (animations, async data):

```rust
use std::time::Duration;

fn handle_events(&mut self) -> io::Result<()> {
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key(key),
            Event::Resize(_, _) => {} // terminal resized, will re-draw
            _ => {}
        }
    }
    Ok(())
}
```

### KeyEventKind

Always check `key.kind == KeyEventKind::Press`. On some platforms crossterm
emits both Press and Release events; without the check, handlers fire twice.

### Mouse Events

Match `Event::Mouse(mouse)` and `mouse.kind` (`MouseEventKind::ScrollUp`,
`ScrollDown`, `Down(button)`, `Up(button)`, `Moved`, etc.).

---

## Styling

### Style Struct

```rust
use ratatui::style::{Style, Color, Modifier};

let style = Style::new()
    .fg(Color::Red)
    .bg(Color::Black)
    .add_modifier(Modifier::BOLD | Modifier::ITALIC);
```

### Colors

`Color::Red`, `Color::LightRed` (named ANSI), `Color::Indexed(208)` (256),
`Color::Rgb(255, 128, 0)` (true color), `Color::Reset` (terminal default).

### Stylize Trait (Method Chaining)

```rust
use ratatui::style::Stylize;

"hello".bold().red().on_blue()           // -> styled Span
Span::raw("text").italic().cyan()        // modify Span
Line::from("header").bold().underlined() // modify Line
```

Modifiers: `bold()`, `italic()`, `underlined()`, `dim()`, `reversed()`,
`crossed_out()`, `slow_blink()`, `rapid_blink()`.

### Text Macros

```rust
use ratatui::macros::{span, line, text};

let s = span!("count: {}", 42);
let l = line!["Name: ", "ratatui".bold().cyan()];
let t = text!["Line one", "Line two".italic()];
```

### Style Composition

Combine with `patch` (patched style overrides set fields):

```rust
let base = Style::new().fg(Color::White).bg(Color::Black);
let highlight = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
let merged = base.patch(highlight); // fg=Yellow, bg=Black, bold
```

---

## Navigation Guide

| Reference File | When to Read |
|---|---|
| `references/widgets.md` | Implementing or debugging a specific widget |
| `references/layout-advanced.md` | Designing complex or nested layouts |
| `references/custom-widgets.md` | Creating a new custom widget |
| `references/patterns.md` | Designing app architecture or adding screens |

---

## Testing

### TestBackend

ratatui provides `TestBackend` for headless rendering:

```rust
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn test_render() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        frame.render_widget("Hello", frame.area());
    }).unwrap();

    let buffer = terminal.backend().buffer().clone();
    assert_eq!(buffer[(0, 0)].symbol(), "H");
}
```

### Snapshot Testing with insta

This project uses `insta`. Combine with `TestBackend`:

```rust
#[test]
fn test_ui_snapshot() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| App::default().render(frame)).unwrap();
    insta::assert_debug_snapshot!(terminal.backend().buffer());
}
```

### Testing Event Handlers

Test as pure logic without a terminal:

```rust
#[test]
fn test_quit_on_q() {
    let mut app = App { should_quit: false };
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert!(app.should_quit);
}
```

### Tips

- Separate rendering and event handling for independent testability.
- Use small `TestBackend` sizes (40x10) to keep snapshots readable.
- Run `cargo insta review` to accept/reject snapshot changes.
- Test edge cases: empty lists, long strings, zero-size areas.
