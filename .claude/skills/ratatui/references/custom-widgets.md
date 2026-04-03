# Custom Widget Authoring (ratatui 0.30.x)

## Widget Trait

Core rendering trait. Consumes `self` by value.

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer) where Self: Sized;
}
```

### Minimal Example

```rust
struct Greeting;

impl Widget for Greeting {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Line::raw("Hello").render(area, buf);
    }
}
```

### Recommended: `impl Widget for &MyWidget`

Avoids consuming the widget -- it can be rendered multiple times.

```rust
struct Label<'a> { text: &'a str }

impl Widget for &Label<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, self.text, Style::default());
    }
}

let label = Label { text: "Status" };
label.render(area_a, buf);
label.render(area_b, buf); // not consumed
```

Built-in impls: `&str`, `String`, `Option<W: Widget>`.

---

## StatefulWidget Trait

For widgets needing mutable state across frames (scroll offset, selection, animation).

```rust
pub trait StatefulWidget {
    type State: ?Sized;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

Render via `Frame::render_stateful_widget` instead of `Frame::render_widget`.

```rust
struct ItemList<'a> { items: &'a [String] }
struct ListState { selected: usize }

impl StatefulWidget for &ItemList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        for (i, item) in self.items.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.bottom() { break; }
            let style = if i == state.selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else { Style::default() };
            buf.set_string(area.x, y, item, style);
        }
    }
}
```

---

## WidgetRef Trait (Unstable)

Behind `unstable-widget-ref` Cargo feature. Renders by `&self`, enabling
`Box<dyn WidgetRef>` for heterogeneous collections.

```rust
// Cargo.toml: ratatui = { version = "0.30", features = ["unstable-widget-ref"] }

pub trait WidgetRef {
    fn render_ref(&self, area: Rect, buf: &mut Buffer);
}
pub trait StatefulWidgetRef {
    type State: ?Sized;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

A blanket impl provides `Widget for &W` when `W: WidgetRef`.

```rust
impl WidgetRef for Badge {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Line::raw(&self.label).render(area, buf);
    }
}

let widgets: Vec<Box<dyn WidgetRef>> = vec![Box::new(a), Box::new(b)];
for w in &widgets { w.render_ref(area, buf); }
```

Only use when you need `dyn` dispatch. Otherwise prefer `impl Widget for &MyWidget`.

---

## Buffer Operations

`Buffer` is a grid of `Cell` values backing all widget rendering.

```rust
pub struct Buffer { pub area: Rect, pub content: Vec<Cell> }
```

### Key Methods

| Method | Signature | Notes |
|--------|-----------|-------|
| `set_string` | `(&mut self, x, y, impl AsRef<str>, impl Into<Style>)` | Write full string |
| `set_stringn` | `(&mut self, x, y, impl AsRef<str>, max_width: usize, impl Into<Style>) -> (u16, u16)` | Truncated; returns end pos |
| `set_line` | `(&mut self, x, y, &Line, max_width: u16) -> (u16, u16)` | Render styled Line |
| `set_span` | `(&mut self, x, y, &Span, max_width: u16) -> (u16, u16)` | Render single Span |
| `set_style` | `(&mut self, area: Rect, impl Into<Style>)` | Style all cells in area |
| `cell` | `(&self, impl Into<Position>) -> Option<&Cell>` | Safe read |
| `cell_mut` | `(&mut self, impl Into<Position>) -> Option<&mut Cell>` | Safe write |

### Indexing

`Index`/`IndexMut` for `impl Into<Position>`. Panics if out of bounds.

```rust
buf[(3, 1)].set_symbol("X");                     // tuple -> Position
buf[Position { x: 3, y: 1 }].set_symbol("X");   // explicit Position

// Safe alternative
if let Some(cell) = buf.cell_mut((3, 1)) {
    cell.set_symbol("X").set_style(Style::default().fg(Color::Red));
}
```

### Common Pattern: Background Fill + Content

```rust
buf.set_style(area, Style::default().bg(Color::DarkGray));
buf.set_string(area.x + 1, area.y, "Title", Style::default().bold());
```

---

## Practical Example

Status bar with label, progress bar, and percentage indicator.

```rust
struct StatusBar<'a> {
    label: &'a str,
    progress: f64, // 0.0..=1.0
}

impl<'a> StatusBar<'a> {
    fn new(label: &'a str, progress: f64) -> Self {
        Self { label, progress: progress.clamp(0.0, 1.0) }
    }
}

impl Widget for &StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() { return; }

        // Background
        buf.set_style(area, Style::default().bg(Color::DarkGray).fg(Color::White));

        // Label (left)
        let label_style = Style::default().fg(Color::White).bold();
        let (end_x, _) = buf.set_stringn(
            area.x + 1, area.y, self.label,
            area.width.saturating_sub(2) as usize, label_style,
        );

        // Progress bar (middle)
        let pct_text = format!("{:>3}%", (self.progress * 100.0) as u16);
        let pct_width = pct_text.len() as u16;
        let bar_start = end_x + 2;
        let bar_end = area.right().saturating_sub(pct_width + 2);

        if bar_start < bar_end {
            let bar_width = bar_end - bar_start;
            let filled = (bar_width as f64 * self.progress) as u16;
            for x in bar_start..bar_end {
                let is_filled = x - bar_start < filled;
                let (sym, color) = if is_filled { ("\u{2588}", Color::Green) } else { ("\u{2591}", Color::Gray) };
                if let Some(cell) = buf.cell_mut((x, area.y)) {
                    cell.set_symbol(sym).set_style(Style::default().fg(color));
                }
            }
        }

        // Percentage (right)
        let pct_x = area.right().saturating_sub(pct_width + 1);
        buf.set_string(pct_x, area.y, &pct_text, Style::default().fg(Color::Yellow));
    }
}

// frame.render_widget(&StatusBar::new("Building", 0.73), bottom_area);
```
