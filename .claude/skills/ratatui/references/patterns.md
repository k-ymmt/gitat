# App Architecture Patterns — ratatui 0.30.x

| Section | Description |
|---|---|
| [Component Pattern](#component-pattern) | Widget + state + event handling bundled |
| [State Management](#state-management) | App struct, run loop, event dispatch |
| [Multi-Screen Navigation](#multi-screen-navigation) | Enum-based screen switching |
| [Error Handling](#error-handling) | color_eyre, panic hooks, terminal restore |
| [Async Integration](#async-integration) | tokio + channels for background tasks |
| [Graceful Shutdown](#graceful-shutdown) | Ctrl+C, cleanup sequence |

---

## Component Pattern

Bundle rendering and event handling into a struct. Implement `Widget` for
`&Component` and add `on_key_press()`. The parent delegates events to the
focused component.

```rust
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

struct Counter { label: &'static str, value: i32 }
impl Counter {
    fn new(label: &'static str) -> Self { Self { label, value: 0 } }
    fn on_key_press(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.value = self.value.saturating_add(1),
            KeyCode::Down | KeyCode::Char('j') => self.value = self.value.saturating_sub(1),
            _ => {}
        }
    }
}
impl Widget for &Counter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        format!("{}: {}", self.label, self.value).render(area, buf);
    }
}
```

---

## State Management

App struct owns a `should_quit` flag and sub-components. The run loop alternates
between drawing and event handling.

```rust
use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::{DefaultTerminal, Frame};

struct App { should_quit: bool, counter: Counter }
impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
    fn render(&self, frame: &mut Frame) {
        frame.render_widget(&self.counter, frame.area());
    }
    fn handle_events(&mut self) -> Result<()> {
        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => self.counter.on_key_press(key),
            }
        }
        Ok(())
    }
}
fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App { should_quit: false, counter: Counter::new("Score") }.run(terminal))
}
```

Event handling notes:
- `as_key_press_event()` filters to press only (avoids double-fire on Windows).
- `is_key_press()` returns `bool` for simple "any key" checks.
- `event::poll(Duration::from_millis(250))?` before `read()` enables periodic
  work (animations, timed updates) between events.

---

## Multi-Screen Navigation

Use an enum for screens; match in both render and event dispatch.

```rust
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Screen { #[default] Home, Settings, Help }

struct App {
    screen: Screen, should_quit: bool,
    home: HomeState, settings: SettingsState,
}

impl App {
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('1') => self.screen = Screen::Home,
            KeyCode::Char('2') => self.screen = Screen::Settings,
            _ => match self.screen {
                Screen::Home => self.home.handle_key(key),
                Screen::Settings => self.settings.handle_key(key),
                Screen::Help => {}
            },
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.screen {
            Screen::Home => self.home.render(area, buf),
            Screen::Settings => self.settings.render(area, buf),
            Screen::Help => "Press 1/2, q to quit".render(area, buf),
        }
    }
}
```

For tab bars, combine with `Tabs` widget and `strum::EnumIter` + `FromRepr`
for cycling (see demo2 example).

---

## Error Handling

### color_eyre (requires `color_eyre` dependency)

Install BEFORE ratatui init so the panic hook chain is ordered correctly:

```rust
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;  // Must come first
    ratatui::run(|terminal| App::new().run(terminal))
}
```

`ratatui::run()` installs its own panic hook that restores the terminal, then
delegates to the previous hook (color_eyre's). Panics will: (1) restore the
terminal, (2) print the color_eyre report.

### Manual init

```rust
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();  // Always runs before error propagation
    result
}
```

Without color_eyre, `ratatui::run()` still installs a panic hook that restores
the terminal. With manual init, ensure `ratatui::restore()` runs before errors
propagate.

---

## Async Integration

### tokio + EventStream (requires `tokio`, `crossterm/event-stream`, `tokio-stream`)

Use manual init/restore with `tokio::select!` to multiplex render ticks,
terminal events, and app messages via `mpsc::channel`.

```rust
use std::time::Duration;
use crossterm::event::{EventStream, KeyCode};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

enum AppEvent { DataLoaded(Vec<String>), Error(String) }

struct App { should_quit: bool, data: Vec<String>, loading: bool }

impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let (tx, mut rx) = mpsc::channel::<AppEvent>(32);
        tokio::spawn({ let tx = tx.clone(); async move {
            // Background work sends results via tx
        }});
        let mut interval = tokio::time::interval(Duration::from_secs_f32(1.0 / 30.0));
        let mut events = EventStream::new();
        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|f| self.render(f))?; }
                Some(Ok(ev)) = events.next() => {
                    if let Some(key) = ev.as_key_press_event() {
                        if key.code == KeyCode::Char('q') { self.should_quit = true; }
                    }
                }
                Some(app_ev) = rx.recv() => match app_ev {
                    AppEvent::DataLoaded(d) => { self.data = d; self.loading = false; }
                    AppEvent::Error(_) => { self.loading = false; }
                },
            }
        }
        Ok(())
    }
    fn render(&self, frame: &mut Frame) { /* ... */ }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = App { should_quit: false, data: vec![], loading: true }
        .run(&mut terminal).await;
    ratatui::restore();
    result
}
```

- Use `ratatui::init()` / `ratatui::restore()` (not `run()`) with async.
- `EventStream` converts terminal events into an async stream.
- `mpsc::channel` decouples background work from the event loop.

---

## Graceful Shutdown

Ctrl+C arrives as `KeyCode::Char('c')` with `KeyModifiers::CONTROL`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn handle_key(&mut self, key: KeyEvent) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        self.should_quit = true;
    }
}
```

With `ratatui::run()`, restoration is automatic. For manual control:

```rust
let mut terminal = ratatui::init();
let result = app.run(&mut terminal);
ratatui::restore();  // Runs even if app.run() returned Err
result?;
```

Prefer `ratatui::run()` for synchronous apps to avoid forgetting restore.
