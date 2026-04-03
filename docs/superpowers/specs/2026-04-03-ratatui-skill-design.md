# Ratatui Skill Design

## Overview

A comprehensive Claude Code skill for building TUI applications with ratatui 0.30.x, placed as a project-local skill for the gitat project.

## Requirements

- **Scope**: Project-local (`.claude/skills/ratatui/`)
- **Purpose**: Comprehensive reference covering all ratatui domains equally
- **Version**: ratatui 0.30.x only (no legacy API coverage)
- **Structure**: SKILL.md with quick start + core concepts; detailed topics in `references/`
- **Language**: English
- **Information sources**: Hybrid approach — ratatui.rs for concepts, repository source + examples for API accuracy

## File Structure

```
.claude/skills/ratatui/
├── SKILL.md                    # Main skill (~400-500 lines)
└── references/
    ├── widgets.md              # All built-in widgets reference (~300-400 lines)
    ├── layout-advanced.md      # Advanced layout patterns (~150-200 lines)
    ├── custom-widgets.md       # Widget/StatefulWidget trait impl (~150-200 lines)
    └── patterns.md             # App architecture patterns (~200-250 lines)
```

## SKILL.md Content

### Frontmatter

```yaml
name: ratatui
description: >
  Comprehensive reference for building TUI applications with ratatui 0.30.x.
  Use when writing ratatui code: creating widgets, layouts, event handling,
  styling, or testing. Triggers: ratatui, TUI, terminal UI, Widget, Layout,
  Constraint, crossterm, TestBackend.
```

### Body Sections (~400-500 lines total)

1. **Quick Start** (~60 lines)
   - Minimal working app using `ratatui::init()` / `ratatui::restore()` convenience functions
   - Includes event loop with crossterm, demonstrating the full app lifecycle in one example
   - Note: crossterm may need to be an explicit `Cargo.toml` dependency for event handling (verify during info gathering)
   - Complete compilable code example

2. **Core Concepts** (~50 lines)
   - Terminal, Frame, Widget trait, Rect relationships
   - Immediate mode rendering model
   - Drawing flow: Terminal::draw() -> closure receives Frame -> frame.render_widget()

3. **Layout System** (~60 lines)
   - Layout::horizontal() / Layout::vertical()
   - Constraint types: Length, Percentage, Min, Max, Fill, Ratio
   - Basic split examples

4. **Common Widgets Overview** (~80 lines)
   - Block, Paragraph, List, Table, Tabs, Gauge — basic usage
   - Selection guide: which widget for which use case
   - Link to `references/widgets.md` for full details

5. **Event Handling** (~50 lines)
   - crossterm poll/read pattern (event loop already shown in Quick Start; this section covers details)
   - KeyEvent matching with pattern matching
   - Mouse events and resize events

6. **Styling** (~40 lines)
   - Style::new(), Color enum, Modifier flags
   - Stylize trait chain notation (e.g., "text".bold().red())
   - Style composition and precedence

7. **Navigation Guide** (~10 lines)
   - When to read each reference file:
     - `references/widgets.md` — when implementing or debugging a specific widget
     - `references/layout-advanced.md` — when designing complex or nested layouts
     - `references/custom-widgets.md` — when creating a new custom widget
     - `references/patterns.md` — when designing app architecture or adding new screens

8. **Testing** (~50 lines)
   - TestBackend for rendering tests
   - insta snapshot testing integration
   - Example test case

## References Content

### references/widgets.md (~400-500 lines)

Per-widget format:
- One-line purpose description
- Basic construction example (code)
- Key methods/options

Widgets covered (complex widgets get more space, simple ones stay terse):
- **Detailed** (~40-50 lines each): Table, Chart (Dataset + Axis), Canvas, List
- **Standard** (~20-30 lines each): Block, Paragraph, Tabs, Scrollbar, BarChart
- **Brief** (~10-15 lines each): Gauge, LineGauge, Sparkline, Clear, Calendar

Table of contents at top for navigation.

### references/layout-advanced.md (~150-200 lines)

- Nested layouts (vertical containing horizontal, etc.)
- Constraint combination patterns and behavior
- Flex layout (Flex::Start, Center, End, SpaceBetween, SpaceAround)
- Margin/Padding handling
- Dynamic sizing patterns

Table of contents at top.

### references/custom-widgets.md (~150-200 lines)

- `Widget` trait implementation (`fn render(self, area: Rect, buf: &mut Buffer)`)
- `StatefulWidget` trait implementation (stateful rendering)
- `WidgetRef` trait (render by reference)
- Direct Buffer manipulation (`buf.set_string`, `buf.set_style`, etc.)
- Practical custom widget example (e.g., status bar with progress)

Table of contents at top.

### references/patterns.md (~200-250 lines)

- **Component pattern**: Widget + State + Action bundled together
- **State management**: App struct for global state
- **Multi-screen**: enum-based screen switching
- **Error handling**: panic hook setup with color_eyre, terminal restoration
- **Async integration**: tokio integration pattern (note: requires adding tokio dependency)
- **Graceful shutdown**: signal handling

Table of contents at top.

## Information Gathering Plan

1. Clone ratatui repo to `.tmp/ratatui`
2. Read ratatui.rs for conceptual structure
3. Study `examples/` for 0.30.0-compatible code patterns
4. Examine `src/` public API for accurate signatures
5. Check CHANGELOG/BREAKING_CHANGES for 0.30.0-specific changes
6. Verify crossterm dependency requirements (re-export vs explicit dependency)
7. Write skill with compilable code examples based on repository sources

## Design Decisions

- **SKILL.md size**: Kept under 500 lines per skill spec recommendation. Contains daily-use information.
- **Progressive disclosure**: Detailed widget reference and advanced topics in separate files, loaded only when needed.
- **Code examples**: Based on repository examples for accuracy. All examples should be compilable with ratatui 0.30.x.
- **Table of contents**: Each reference file includes a TOC so Claude can identify needed sections without reading the entire file.
- **No legacy coverage**: Only 0.30.x API. No deprecated methods or migration guides from older versions.
