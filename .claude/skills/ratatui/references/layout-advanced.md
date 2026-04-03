# Advanced Layout Patterns (ratatui 0.30.x)

## Table of Contents

- [Nested Layouts](#nested-layouts)
- [Constraint Patterns](#constraint-patterns)
- [Flex Layout](#flex-layout)
- [Margin, Padding, Spacing](#margin-padding-spacing)
- [Dynamic Sizing](#dynamic-sizing)
- [Layout Macros](#layout-macros)

## Nested Layouts

### Vertical containing Horizontal

```rust
let [header, body, footer] = Layout::vertical([
    Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1),
]).areas(area);

let [sidebar, content, panel] = Layout::horizontal([
    Constraint::Length(20), Constraint::Fill(1), Constraint::Length(30),
]).areas(body);
```

### Rect::layout Shorthand

```rust
let [header, body] = area.layout(&Layout::vertical([
    Constraint::Length(3), Constraint::Fill(1),
]));
```

## Constraint Patterns

### Priority System

Constraints resolve highest priority first:

| Priority | Constraint        | Behavior                      |
|----------|-------------------|-------------------------------|
| 1        | `Min(u16)`        | At least N cells              |
| 2        | `Max(u16)`        | At most N cells               |
| 3        | `Length(u16)`     | Exactly N cells (best effort) |
| 4        | `Percentage(u16)` | % of total available          |
| 5        | `Ratio(u32,u32)`  | Fraction of total available   |
| 6        | `Fill(u16)`       | Proportional fill of excess   |

### Common Combinations

```rust
// Fixed header + flexible body
[Constraint::Length(3), Constraint::Fill(1)]
// Weighted split 1:2:1
[Constraint::Fill(1), Constraint::Fill(2), Constraint::Fill(1)]
// Min-bounded flexible
[Constraint::Min(10), Constraint::Fill(1)]
// Max-bounded column
[Constraint::Max(40), Constraint::Fill(1)]
```

### Batch Construction

```rust
Constraint::from_lengths([10, 20, 10])
Constraint::from_fills([1, 2, 1])
Constraint::from_percentages([25, 50, 25])
Constraint::from_ratios([(1, 4), (1, 2), (1, 4)])
Constraint::from_mins([0, 100, 0])
Constraint::from_maxes([30, 170])
```

## Flex Layout

`Flex` controls excess space distribution. Default is `Flex::Start`.

| Variant        | Description                                               |
|----------------|-----------------------------------------------------------|
| `Legacy`       | Excess to last lowest-priority constraint (pre-0.26)      |
| `Start`        | Pack areas toward start (default)                         |
| `End`          | Pack areas toward end                                     |
| `Center`       | Center areas in container                                 |
| `SpaceBetween` | Excess between areas, none at edges                       |
| `SpaceAround`  | Inner gaps 2x outer gaps (CSS flexbox model, changed 0.30)|
| `SpaceEvenly`  | Equal gaps everywhere (new in 0.30)                       |

```rust
let [a, b, c] = Layout::horizontal([
    Constraint::Length(10), Constraint::Length(10), Constraint::Length(10),
]).flex(Flex::Center).areas(area);
```

### SpaceAround vs SpaceEvenly (0.30)

```text
SpaceAround (3 items, 24px excess):
  [3px] [AAAA] [6px] [BBBB] [6px] [CCCC] [3px]
         outer = half of inner

SpaceEvenly (3 items, 24px excess):
  [6px] [AAAA] [6px] [BBBB] [6px] [CCCC] [6px]
         all gaps equal
```

## Margin, Padding, Spacing

### Layout Margin

```rust
Layout::vertical([Constraint::Fill(1); 2])
    .margin(1)               // 1 cell on all sides
    .areas::<2>(area);

Layout::horizontal([Constraint::Fill(1); 2])
    .horizontal_margin(2)    // 2 cells left + right
    .vertical_margin(1)      // 1 cell top + bottom
    .areas::<2>(area);
```

### Block Padding

```rust
let block = Block::bordered().padding(Padding::new(1, 1, 0, 0)); // left, right, top, bottom
let inner = block.inner(area); // usable area inside the block
```

### Spacing (Gap / Overlap)

`Spacing::Space(n)` inserts gap; `Spacing::Overlap(n)` shares borders.

```rust
// 1-cell gap between rows
Layout::vertical([Constraint::Length(3); 3])
    .spacing(1)
    .areas::<3>(area);

// Overlapping borders (shared 1-cell border)
Layout::vertical([Constraint::Fill(1); 3])
    .spacing(Spacing::Overlap(1))
    .areas::<3>(area);
```

Negative integer values convert to `Overlap`:
```rust
Layout::vertical([Constraint::Fill(1); 2]).spacing(-1); // Spacing::Overlap(1)
```

## Dynamic Sizing

### Rect::centered for Popups/Dialogs (new in 0.30)

```rust
// Center a 60%-wide, 10-row popup
let popup = area.centered(Constraint::Percentage(60), Constraint::Length(10));

// Horizontal only
let banner = area.centered_horizontally(Constraint::Length(40));

// Vertical only
let middle = area.centered_vertically(Constraint::Ratio(1, 3));
```

Internally uses `Layout` with `Flex::Center`.

### Responsive Patterns

```rust
let [left, right] = if area.width >= 80 {
    Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)])
} else {
    Layout::horizontal([Constraint::Fill(1), Constraint::Length(0)])
}.areas(area);
```

## Layout Macros

`ratatui-macros` (enabled by default) provides shorthand syntax.

### Constraint Shorthand

| Syntax  | Expands to                 |
|---------|----------------------------|
| `==50`  | `Constraint::Length(50)`   |
| `==30%` | `Constraint::Percentage(30)` |
| `>=3`   | `Constraint::Min(3)`      |
| `<=10`  | `Constraint::Max(10)`     |
| `==1/3` | `Constraint::Ratio(1, 3)` |
| `*=1`   | `Constraint::Fill(1)`     |

### constraints!, vertical!, horizontal!

```rust
let c = constraints![==50, ==30%, >=3, <=1, ==1/2, *=1];
let c = constraints![==10; 5]; // repeat syntax

let [header, body] = vertical![==3, *=1].areas(area);
let [sidebar, main] = horizontal![>=20, *=1].areas(area);
let [a, b] = horizontal![==10, ==10].flex(Flex::Center).areas(area);
```
