# Ratatui 0.30.x Widget Reference

## Table of Contents

- [Text Types as Widgets](#text-types-as-widgets)
- [Block](#block) -- borders, titles, padding
- [Paragraph](#paragraph) -- styled, wrapped, scrollable text
- [List](#list) -- selectable list of items
- [Table](#table) -- multi-column data grid
- [Chart](#chart) -- line/scatter graphs
- [Canvas](#canvas) -- arbitrary shape drawing
- [Tabs](#tabs) -- tab bar
- [Scrollbar](#scrollbar) -- scrollbar indicator
- [BarChart](#barchart) -- grouped bar charts
- [Gauge](#gauge) -- progress bar
- [LineGauge](#linegauge) -- progress line
- [Sparkline](#sparkline) -- mini data visualization
- [Clear](#clear) -- clears an area
- [Calendar (Monthly)](#calendar-monthly) -- month display

---

## Text Types as Widgets

`&str`, `String`, `Span`, `Line`, and `Text` all implement `Widget` directly. You can render them without wrapping in `Paragraph` when wrapping/scrolling is not needed:

```rust
frame.render_widget("hello world", area);
frame.render_widget(Line::from("styled").bold(), area);
```

---

## Block

Container widget that draws borders, titles, and padding around other widgets. Almost every widget accepts `.block()` to wrap itself in a `Block`.

**Constructors:**
- `Block::new()` -- no borders
- `Block::bordered()` -- all four borders enabled

**Key methods:**
- `.title(Into<Line>)` -- add a title (top by default)
- `.title_top(Into<Line>)` -- add a title at the top
- `.title_bottom(Into<Line>)` -- add a title at the bottom
- `.title_alignment(Alignment)` -- default alignment for all titles
- `.title_style(Style)` -- style for all titles
- `.borders(Borders)` -- which borders to draw
- `.border_type(BorderType)` -- border symbols (Plain, Rounded, Double, Thick, QuadrantOutside, QuadrantInside)
- `.border_style(Style)` -- style of the borders
- `.padding(Padding)` -- internal padding
- `.style(Style)` -- base style
- `.inner(Rect) -> Rect` -- calculate the inner area after borders/padding

**0.30 breaking change:** The `block::Title` struct was removed. Titles now accept `Into<Line>` directly. Use `Line::from("Title").centered()` for alignment.

```rust
use ratatui::layout::Alignment;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, BorderType, Padding};

let block = Block::bordered()
    .title("Top Title")
    .title_bottom(Line::from("Bottom").centered())
    .border_type(BorderType::Rounded)
    .border_style(Style::new().blue())
    .padding(Padding::horizontal(1))
    .style(Style::new().on_black());

// Use .inner() to calculate the area available for content
let inner_area = block.inner(area);
frame.render_widget(block, area);
```

---

## Paragraph

Displays styled, wrapped, and scrollable text.

**Constructor:** `Paragraph::new(Into<Text>)`

**Key methods:**
- `.block(Block)` -- wrap in a block
- `.style(Style)` -- base style
- `.alignment(Alignment)` -- text alignment
- `.left_aligned()` / `.centered()` / `.right_aligned()` -- shorthand alignment
- `.wrap(Wrap { trim })` -- enable word wrapping; `trim: true` trims leading whitespace
- `.scroll((u16, u16))` -- scroll offset as `(vertical, horizontal)`

```rust
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

let text = vec![
    Line::from(vec![
        Span::raw("Hello "),
        Span::styled("world", Style::new().green().italic()),
    ]),
    Line::from("Second line".red()),
];

let paragraph = Paragraph::new(text)
    .block(Block::bordered().title("Output"))
    .wrap(Wrap { trim: true })
    .scroll((scroll_offset, 0))
    .style(Style::new().white().on_black());

frame.render_widget(paragraph, area);
```

---

## List

Selectable list of items. Supports stateful selection via `ListState`.

**Constructor:** `List::new(items)` -- items can be `Vec<ListItem>`, `Vec<&str>`, or any `IntoIterator<Item: Into<ListItem>>`

**Key methods:**
- `.block(Block)` -- wrap in a block
- `.style(Style)` -- base style
- `.highlight_style(Style)` -- style for the selected item
- `.highlight_symbol(Into<Line>)` -- symbol shown before the selected item (accepts `Into<Line>` since 0.30)
- `.repeat_highlight_symbol(bool)` -- repeat symbol on multi-line items
- `.direction(ListDirection)` -- `TopToBottom` (default) or `BottomToTop`
- `.highlight_spacing(HighlightSpacing)` -- control spacing when nothing is selected

**State:** `ListState` tracks selection and scroll offset.
- `ListState::default()` -- no selection
- `.select(Some(index))` / `.select(None)` -- set or clear selection
- `.selected() -> Option<usize>` -- get current selection
- `.select_next()` / `.select_previous()` -- move selection

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, List, ListItem, ListState};

let items = vec![
    ListItem::new("Item 1"),
    ListItem::new("Item 2").style(Style::new().bold()),
    ListItem::new("Item 3"),
];

let list = List::new(items)
    .block(Block::bordered().title("List"))
    .highlight_style(Style::new().reversed())
    .highlight_symbol(">> ")
    .repeat_highlight_symbol(true);

// Stateful rendering
let mut state = ListState::default().with_selected(Some(0));
frame.render_stateful_widget(list, area, &mut state);
```

**Navigation pattern:**
```rust
// In key handler:
KeyCode::Down => state.select_next(),
KeyCode::Up => state.select_previous(),
```

---

## Table

Multi-column data grid with headers, footers, and stateful row/column/cell selection.

**Constructor:** `Table::new(rows, widths)` -- `rows: IntoIterator<Item: Into<Row>>`, `widths: IntoIterator<Item: Into<Constraint>>`

**Key methods:**
- `.header(Row)` -- header row (always visible at top)
- `.footer(Row)` -- footer row (always visible at bottom)
- `.widths(IntoIterator<Item: Into<Constraint>>)` -- column width constraints
- `.column_spacing(u16)` -- spacing between columns
- `.block(Block)` -- wrap in a block
- `.style(Style)` -- base style
- `.row_highlight_style(Style)` -- style for selected row
- `.column_highlight_style(Style)` -- style for selected column
- `.cell_highlight_style(Style)` -- style for selected cell
- `.highlight_symbol(Into<Text>)` -- symbol in front of selected row
- `.highlight_spacing(HighlightSpacing)` -- control spacing

**Supporting types:**
- `Row::new(cells)` -- cells are `IntoIterator<Item: Into<Cell>>`; methods: `.height(u16)`, `.bottom_margin(u16)`, `.style(Style)`
- `Cell::new(Into<Text>)` -- or `Cell::from("text")`; methods: `.style(Style)`
- `TableState` -- tracks selected row/column and scroll offset

```rust
use ratatui::layout::Constraint;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Cell, Row, Table, TableState};

let header = Row::new(vec!["Name", "Size", "Modified"])
    .style(Style::new().bold())
    .bottom_margin(1);

let rows = vec![
    Row::new(vec!["file.rs", "1.2K", "2025-01-15"]),
    Row::new(vec!["main.rs", "3.4K", "2025-01-14"]),
];

let widths = [
    Constraint::Min(10),
    Constraint::Length(8),
    Constraint::Length(12),
];

let table = Table::new(rows, widths)
    .header(header)
    .block(Block::bordered().title("Files"))
    .row_highlight_style(Style::new().reversed())
    .highlight_symbol(">> ");

let mut state = TableState::default().with_selected(Some(0));
frame.render_stateful_widget(table, area, &mut state);
```

**Navigation pattern:**
```rust
KeyCode::Down => state.select_next(),
KeyCode::Up => state.select_previous(),
```

---

## Chart

Line and scatter graphs plotted on a cartesian coordinate system with labeled axes.

**Constructor:** `Chart::new(Vec<Dataset>)`

**Key methods:**
- `.x_axis(Axis)` -- configure the X axis
- `.y_axis(Axis)` -- configure the Y axis
- `.block(Block)` -- wrap in a block
- `.style(Style)` -- base style
- `.legend_position(Option<LegendPosition>)` -- position of the legend, or `None` to hide
- `.hidden_legend_constraints((Constraint, Constraint))` -- minimum chart size to show legend

**Axis:**
- `Axis::default()` -- empty axis
- `.title(Into<Line>)` -- axis label
- `.bounds([f64; 2])` -- min/max values
- `.labels(IntoIterator<Item: Into<Line>>)` -- tick labels
- `.style(Style)` -- axis style

**Dataset:**
- `Dataset::default()` -- empty dataset
- `.name(Into<Line>)` -- legend name
- `.data(&[(f64, f64)])` -- data points
- `.marker(Marker)` -- symbol type (`Dot`, `Braille`, `Block`, `HalfBlock`, `Bar`)
- `.graph_type(GraphType)` -- `Line` or `Scatter`
- `.style(Style)` -- dataset style

```rust
use ratatui::style::{Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Block, Chart, Dataset, GraphType};

let data1: Vec<(f64, f64)> = vec![(0.0, 1.0), (1.0, 3.0), (2.0, 2.0), (3.0, 5.0)];
let data2: Vec<(f64, f64)> = vec![(0.0, 2.0), (1.0, 1.0), (2.0, 4.0), (3.0, 3.0)];

let datasets = vec![
    Dataset::default()
        .name("Series 1")
        .data(&data1)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::new().cyan()),
    Dataset::default()
        .name("Series 2")
        .data(&data2)
        .marker(Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::new().yellow()),
];

let chart = Chart::new(datasets)
    .block(Block::bordered().title("Chart"))
    .x_axis(
        Axis::default()
            .title("X")
            .bounds([0.0, 3.0])
            .labels(["0", "1", "2", "3"]),
    )
    .y_axis(
        Axis::default()
            .title("Y")
            .bounds([0.0, 5.0])
            .labels(["0", "2.5", "5"]),
    );

frame.render_widget(chart, area);
```

---

## Canvas

Freeform drawing area for arbitrary shapes. Uses a closure for drawing via a `Context`.

**Constructor:** `Canvas::default()`

**Key methods:**
- `.paint(Fn(&mut Context))` -- drawing closure
- `.x_bounds([f64; 2])` -- horizontal coordinate range
- `.y_bounds([f64; 2])` -- vertical coordinate range
- `.marker(Marker)` -- resolution (`Braille`, `Dot`, `Block`, `HalfBlock`, `Bar`)
- `.background_color(Color)` -- canvas background
- `.block(Block)` -- wrap in a block

**Context methods (inside `.paint()`):**
- `ctx.draw(&shape)` -- draw any `impl Shape`
- `ctx.print(x, y, Into<Line>)` -- print text at coordinates
- `ctx.layer()` -- start a new drawing layer

**Built-in shapes:**
- `canvas::Line { x1, y1, x2, y2, color }` -- line between two points
- `canvas::Rectangle { x, y, width, height, color }` -- rectangle outline
- `canvas::Circle { x, y, radius, color }` -- circle outline
- `canvas::Map { resolution, color }` -- world map
- `canvas::Points { coords, color }` -- scatter points

```rust
use ratatui::style::Color;
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Circle, Line, Rectangle};
use ratatui::widgets::Block;

let canvas = Canvas::default()
    .block(Block::bordered().title("Canvas"))
    .x_bounds([0.0, 100.0])
    .y_bounds([0.0, 100.0])
    .marker(Marker::Braille)
    .paint(|ctx| {
        ctx.draw(&Rectangle {
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
            color: Color::Yellow,
        });
        ctx.draw(&Circle {
            x: 70.0,
            y: 50.0,
            radius: 20.0,
            color: Color::Cyan,
        });
        ctx.draw(&Line {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
            color: Color::Green,
        });
        ctx.print(50.0, 50.0, "Center".bold());
    });

frame.render_widget(canvas, area);
```

---

## Tabs

Horizontal tab bar with a selected tab indicator.

**Constructor:** `Tabs::new(items)` -- items are `IntoIterator<Item: Into<Line>>`

**Key methods:**
- `.select(usize)` -- set the selected tab index
- `.block(Block)` -- wrap in a block
- `.style(Style)` -- base style
- `.highlight_style(Style)` -- style for the selected tab
- `.divider(Into<Span>)` -- separator between tabs (default: `|`)
- `.padding(Into<Line>, Into<Line>)` -- left and right padding for each tab

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Tabs};

let titles = vec!["Overview", "Details", "Settings"];
let tabs = Tabs::new(titles)
    .block(Block::bordered().title("Navigation"))
    .select(selected_tab)
    .style(Style::new().white())
    .highlight_style(Style::new().yellow().bold())
    .divider(" | ");

frame.render_widget(tabs, area);
```

---

## Scrollbar

Visual scrollbar indicator. A stateful widget paired with `ScrollbarState`.

**Constructor:** `Scrollbar::new(ScrollbarOrientation)` -- `VerticalRight`, `VerticalLeft`, `HorizontalBottom`, `HorizontalTop`

**Key methods:**
- `.begin_symbol(Option<&str>)` -- arrow at start (e.g., `Some("^")`)
- `.end_symbol(Option<&str>)` -- arrow at end (e.g., `Some("v")`)
- `.track_symbol(Option<&str>)` -- track character
- `.thumb_symbol(&str)` -- thumb character
- `.thumb_style(Style)` / `.track_style(Style)` -- styling

**State:** `ScrollbarState::new(content_length)` -- total number of scrollable items
- `.position(usize)` -- current scroll position
- `.viewport_content_length(usize)` -- visible items count

```rust
use ratatui::layout::Margin;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
    .begin_symbol(Some("^"))
    .end_symbol(Some("v"));

let mut scrollbar_state = ScrollbarState::new(total_items)
    .position(current_position);

// Render inside the content area with a margin to avoid overlap
frame.render_stateful_widget(
    scrollbar,
    area.inner(Margin { vertical: 1, horizontal: 0 }),
    &mut scrollbar_state,
);
```

---

## BarChart

Vertical or horizontal bar charts, optionally grouped.

**Constructor:** `BarChart::default()` or `BarChart::new(bars)`

**Key methods:**
- `.data(&[(&str, u64)])` -- add bars from slice (also accepts `BarGroup`)
- `.bar_width(u16)` -- width of each bar
- `.bar_gap(u16)` -- gap between bars in a group
- `.group_gap(u16)` -- gap between groups
- `.bar_style(Style)` -- default bar style
- `.value_style(Style)` -- style for the value text
- `.label_style(Style)` -- style for bar labels
- `.direction(Direction)` -- `Vertical` (default) or `Horizontal`
- `.max(u64)` -- max value (auto-detected if not set)
- `.block(Block)` -- wrap in a block

**Supporting types:** `Bar::with_label(label, value)`, `BarGroup::new(bars)`

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block};

let chart = BarChart::default()
    .block(Block::bordered().title("Downloads"))
    .bar_width(5)
    .bar_gap(2)
    .group_gap(3)
    .bar_style(Style::new().cyan())
    .value_style(Style::new().white().bold())
    .data(&[("Mon", 12), ("Tue", 28), ("Wed", 15)])
    .data(BarGroup::new([
        Bar::with_label("Thu", 22),
        Bar::with_label("Fri", 35),
    ]));

frame.render_widget(chart, area);
```

---

## Gauge

Full-width progress bar with a centered label.

**Constructor:** `Gauge::default()`

**Key methods:**
- `.percent(u16)` -- set progress (0-100, panics otherwise)
- `.ratio(f64)` -- set progress (0.0-1.0, panics otherwise)
- `.label(Into<Span>)` -- custom label (defaults to percentage text)
- `.gauge_style(Style)` -- style for the bar
- `.use_unicode(bool)` -- higher-precision rendering
- `.block(Block)` -- wrap in a block

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Gauge};

let gauge = Gauge::default()
    .block(Block::bordered().title("Progress"))
    .gauge_style(Style::new().green())
    .percent(42)
    .use_unicode(true);

frame.render_widget(gauge, area);
```

---

## LineGauge

Thin single-line progress bar with a label on the left.

**Constructor:** `LineGauge::default()`

**Key methods:**
- `.ratio(f64)` -- progress (0.0-1.0)
- `.label(Into<Line>)` -- custom label
- `.filled_symbol(&str)` -- character for filled portion
- `.unfilled_symbol(&str)` -- character for unfilled portion
- `.filled_style(Style)` / `.unfilled_style(Style)` -- styling
- `.block(Block)` -- wrap in a block

Note: `.line_set()` is deprecated in 0.30; use `.filled_symbol()` and `.unfilled_symbol()` instead.

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, LineGauge};

let line_gauge = LineGauge::default()
    .block(Block::bordered().title("Upload"))
    .ratio(0.65)
    .filled_style(Style::new().green())
    .unfilled_style(Style::new().dark_gray())
    .label("65%");

frame.render_widget(line_gauge, area);
```

---

## Sparkline

Minimal bar-chart visualization in a single row. Good for inline data trends.

**Constructor:** `Sparkline::default()`

**Key methods:**
- `.data(&[u64])` -- values to display
- `.max(u64)` -- maximum value (auto-detected if not set)
- `.direction(RenderDirection)` -- `LeftToRight` (default) or `RightToLeft`
- `.bar_set(symbols::bar::Set)` -- bar characters
- `.style(Style)` -- bar style
- `.absent_value_style(Style)` -- style for `None` values (when using `Option<u64>` data)
- `.absent_value_symbol(&str)` -- symbol for absent values
- `.block(Block)` -- wrap in a block

```rust
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Sparkline};

let sparkline = Sparkline::default()
    .block(Block::bordered().title("Traffic"))
    .data(&[0, 2, 3, 4, 1, 4, 10, 8, 5, 3])
    .max(10)
    .style(Style::new().cyan());

frame.render_widget(sparkline, area);
```

---

## Clear

Resets an area of the buffer to allow overdrawing. Commonly used for popups.

```rust
use ratatui::widgets::{Block, Clear};

// Clear the area, then draw a popup on top
frame.render_widget(Clear, popup_area);
frame.render_widget(Block::bordered().title("Popup"), popup_area);
```

Note: `Clear` cannot clear the terminal on the first render. Use `Terminal::clear()` for that.

---

## Calendar (Monthly)

Displays a single-month calendar view. Requires the `widget-calendar` feature (enabled by `all-widgets`).

**Constructor:** `Monthly::new(date, events)` -- `date: time::Date`, `events: impl DateStyler`

**Key methods:**
- `.show_weekdays_header(Style)` -- display weekday abbreviations
- `.show_month_header(Style)` -- display month/year header
- `.show_surrounding(Style)` -- fill in days from adjacent months
- `.default_style(Style)` -- base day style
- `.block(Block)` -- wrap in a block

```rust
// Cargo.toml: ratatui = { version = "0.30", features = ["widget-calendar"] }
use time::Date;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::widgets::Block;

let date = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
let events = CalendarEventStore::default();
let calendar = Monthly::new(date, &events)
    .show_weekdays_header(Style::new().bold())
    .show_month_header(Style::new().italic())
    .block(Block::bordered());

frame.render_widget(calendar, area);
```
