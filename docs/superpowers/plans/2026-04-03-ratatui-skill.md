# Ratatui Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a comprehensive project-local Claude Code skill for ratatui 0.30.x at `.claude/skills/ratatui/`.

**Architecture:** SKILL.md (~400-500 lines) provides quick start, core concepts, and daily-use reference. Four reference files under `references/` provide detailed widget catalog, advanced layout, custom widget authoring, and app architecture patterns. All code examples must be verified against ratatui 0.30.x source.

**Tech Stack:** ratatui 0.30.x, crossterm (event handling), Rust edition 2024

**Spec:** `docs/superpowers/specs/2026-04-03-ratatui-skill-design.md`

---

## Task 1: Clone ratatui repository and gather information

**Files:**
- Create: `.tmp/ratatui/` (git clone, gitignored)

This task clones the ratatui repository and gathers all information needed to write accurate skill content. All subsequent tasks depend on findings from this research.

- [ ] **Step 1: Ensure .tmp is gitignored**

Check `.gitignore` includes `.tmp/`. If not, add it.

- [ ] **Step 2: Clone ratatui repo**

```bash
git clone --depth 1 https://github.com/ratatui/ratatui.git .tmp/ratatui
```

- [ ] **Step 3: Identify ratatui version and crossterm dependency**

Read `.tmp/ratatui/Cargo.toml` to find:
- The current ratatui version (should be 0.30.x)
- Whether crossterm is a direct dependency or re-exported
- What features are available

The ratatui repository uses a workspace structure. The main crate Cargo.toml is at `.tmp/ratatui/ratatui/Cargo.toml`. Read both the workspace root and the main crate Cargo.toml.

Check if `ratatui::crossterm` re-export exists:

```bash
grep -r "pub use crossterm" .tmp/ratatui/ratatui/src/ || grep -r "pub mod crossterm" .tmp/ratatui/ratatui/src/
```

Document findings for use in all subsequent tasks.

- [ ] **Step 4: Study examples directory**

```bash
ls .tmp/ratatui/examples/
```

Read key examples to understand idiomatic patterns:
- Minimal app example (look for `hello_world` or `minimal`)
- Examples demonstrating each major widget
- Examples showing layout patterns
- Examples showing event handling

Extract compilable code patterns from examples.

- [ ] **Step 5: Study public API surface**

Key files to examine in `src/`:
- `lib.rs` — re-exports, `init()`, `restore()`
- `terminal.rs` — Terminal struct, draw method
- `frame.rs` — Frame struct, render_widget
- `layout.rs` / `layout/` — Layout, Constraint, Flex, Rect
- `widgets/` — all widget implementations and their public API
- `style.rs` / `style/` — Style, Color, Modifier, Stylize trait
- `buffer.rs` — Buffer struct for custom widgets
- `text.rs` / `text/` — Line, Span, Text

Record exact method signatures and trait definitions.

- [ ] **Step 6: Check CHANGELOG for 0.30.0 specifics**

```bash
head -200 .tmp/ratatui/CHANGELOG.md
```

Note any breaking changes or new APIs in 0.30.x.

- [ ] **Step 7: Fetch ratatui.rs concepts page**

Use WebFetch to read `https://ratatui.rs/concepts/` and `https://ratatui.rs/tutorials/` for conceptual framing. Use these for structure/explanations but verify all API details against source.

If WebFetch is unavailable, skip this step. The cloned repository source and examples from Steps 3-6 provide sufficient information.

- [ ] **Step 8: Commit .gitignore update (if modified)**

If .gitignore was modified in Step 1, commit:

```bash
git add .gitignore
git commit -m "Add .tmp to gitignore"
```

If no changes were needed, skip this step.

**Parallelism note:** Tasks 2-6 are independent of each other and may be executed in parallel. All depend only on Task 1. Task 7 depends on all of Tasks 2-6.

---

## Task 2: Write SKILL.md

**Files:**
- Create: `.claude/skills/ratatui/SKILL.md`

**Depends on:** Task 1 (research findings)

Write the main skill file following the spec. Target ~400-500 lines. All code examples must use API verified in Task 1.

- [ ] **Step 1: Create skill directory**

```bash
mkdir -p .claude/skills/ratatui/references
```

- [ ] **Step 2: Write SKILL.md frontmatter and Quick Start section**

Write frontmatter with `name` and `description` fields per spec.

Write Quick Start (~60 lines):
- Minimal working app using `ratatui::init()` / `ratatui::restore()` (or the verified equivalent from Task 1)
- Include event loop with crossterm
- Note crossterm dependency requirement if needed (based on Task 1 Step 3 findings)
- Complete compilable example

- [ ] **Step 3: Write Core Concepts section**

~50 lines covering:
- Terminal, Frame, Widget trait, Rect relationships
- Immediate mode rendering model
- Drawing flow: `Terminal::draw()` -> closure -> `frame.render_widget()`

- [ ] **Step 4: Write Layout System section**

~60 lines covering:
- `Layout::horizontal()` / `Layout::vertical()`
- All Constraint types: Length, Percentage, Min, Max, Fill, Ratio
- Basic split code example

- [ ] **Step 5: Write Common Widgets Overview section**

~80 lines covering:
- Block, Paragraph, List, Table, Tabs, Gauge — basic usage with short code snippets
- Selection guide: which widget for which use case
- Link: "For full widget reference, see `references/widgets.md`"

- [ ] **Step 6: Write Event Handling section**

~50 lines covering:
- crossterm `poll`/`read` pattern (reference Quick Start for full loop)
- KeyEvent matching with pattern matching examples
- Mouse events and resize events

- [ ] **Step 7: Write Styling section**

~40 lines covering:
- `Style::new()`, `Color` enum, `Modifier` flags
- Stylize trait chain notation (`.bold().red()`)
- Style composition and precedence rules

- [ ] **Step 8: Write Navigation Guide section**

~10 lines listing when to read each reference file:
- `references/widgets.md` — implementing or debugging a specific widget
- `references/layout-advanced.md` — complex or nested layouts
- `references/custom-widgets.md` — creating a new custom widget
- `references/patterns.md` — designing app architecture or adding screens

- [ ] **Step 9: Write Testing section**

~50 lines covering:
- TestBackend for rendering tests
- insta snapshot testing integration
- Complete example test case

- [ ] **Step 10: Verify line count and adjust**

Count total lines. If over 500, trim verbose sections. If under 350, add more useful examples.

- [ ] **Step 11: Commit**

```bash
git add .claude/skills/ratatui/SKILL.md
git commit -m "feat: add ratatui skill SKILL.md with core reference"
```

---

## Task 3: Write references/widgets.md

**Files:**
- Create: `.claude/skills/ratatui/references/widgets.md`

**Depends on:** Task 1 (research findings)

Write the comprehensive widget reference. Target ~300-400 lines. Per-widget format: one-line purpose, code example, key methods.

- [ ] **Step 1: Write Table of Contents and Detailed widgets**

Write TOC at top listing all widgets.

Write detailed entries (~40-50 lines each) for:
- **Table** — headers, rows, widths, highlight state, TableState
- **Chart** — Dataset, Axis, chart types, data points
- **Canvas** — painter API, Shape trait, built-in shapes
- **List** — items, selection, ListState, highlight symbol

Use code examples verified against `.tmp/ratatui/` source.

- [ ] **Step 2: Write Standard widgets**

~20-30 lines each for:
- **Block** — title, borders, border type, padding
- **Paragraph** — text wrapping, alignment, scroll
- **Tabs** — tab labels, selected, highlight style
- **Scrollbar** — orientation, position, ScrollbarState
- **BarChart** — bars, values, labels, max width

- [ ] **Step 3: Write Brief widgets**

~10-15 lines each for:
- **Gauge** — ratio/percentage, label
- **LineGauge** — similar to Gauge but line-based
- **Sparkline** — data points, max value
- **Clear** — clear an area
- **Calendar** — date display (if available in 0.30.x; skip if not)

- [ ] **Step 4: Verify line count and adjust**

Target 300-400 lines. Trim or expand as needed.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/ratatui/references/widgets.md
git commit -m "feat: add ratatui widget reference"
```

---

## Task 4: Write references/layout-advanced.md

**Files:**
- Create: `.claude/skills/ratatui/references/layout-advanced.md`

**Depends on:** Task 1 (research findings)

Target ~150-200 lines.

- [ ] **Step 1: Write TOC and nested layouts**

Write TOC at top.

Cover nested layouts with code example:
- Vertical split containing horizontal splits
- Three-column layout with fixed sidebar

- [ ] **Step 2: Write Constraint patterns and Flex**

- Constraint combination patterns and real behavior
- Flex layout: `Flex::Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`
- Code example showing Flex usage

- [ ] **Step 3: Write Margin/Padding and dynamic sizing**

- Margin and Padding on blocks and layouts
- Dynamic sizing patterns (responsive to terminal size)
- Code example

- [ ] **Step 4: Verify line count and commit**

Target 150-200 lines.

```bash
git add .claude/skills/ratatui/references/layout-advanced.md
git commit -m "feat: add advanced layout reference"
```

---

## Task 5: Write references/custom-widgets.md

**Files:**
- Create: `.claude/skills/ratatui/references/custom-widgets.md`

**Depends on:** Task 1 (research findings)

Target ~150-200 lines.

- [ ] **Step 1: Write TOC and Widget trait**

Write TOC at top.

Cover `Widget` trait implementation:
- Trait signature: `fn render(self, area: Rect, buf: &mut Buffer)`
- Minimal example

- [ ] **Step 2: Write StatefulWidget and WidgetRef**

- `StatefulWidget` trait with state parameter
- `WidgetRef` trait for rendering by reference
- When to use each

- [ ] **Step 3: Write Buffer operations and practical example**

- Direct Buffer manipulation: `buf.set_string()`, `buf.set_style()`, `buf.cell_mut()`
- Complete practical example: a custom status bar widget with progress indicator

- [ ] **Step 4: Verify line count and commit**

Target 150-200 lines.

```bash
git add .claude/skills/ratatui/references/custom-widgets.md
git commit -m "feat: add custom widget authoring reference"
```

---

## Task 6: Write references/patterns.md

**Files:**
- Create: `.claude/skills/ratatui/references/patterns.md`

**Depends on:** Task 1 (research findings)

Target ~200-250 lines.

- [ ] **Step 1: Write TOC and Component pattern**

Write TOC at top.

Cover the Component pattern:
- Widget + State + Action bundled together
- Code example showing a component struct with `handle_event()` and `render()` methods

- [ ] **Step 2: Write State management and Multi-screen**

- App struct for global state management
- Code example of App with multiple fields

- enum-based screen switching pattern
- Code example with Screen enum and match-based rendering

- [ ] **Step 3: Write Error handling**

- panic hook setup with `color_eyre` (note: requires adding `color_eyre` dependency)
- Terminal restoration on panic
- `init_error_hooks()` pattern
- Code example

- [ ] **Step 4: Write Async integration and Graceful shutdown**

- tokio integration pattern (note: requires adding `tokio` dependency)
- Spawning background tasks that send events via channels
- Code example

- Signal handling for graceful shutdown
- `ctrlc` or crossterm Ctrl+C detection
- Cleanup sequence

- [ ] **Step 5: Verify line count and commit**

Target 200-250 lines.

```bash
git add .claude/skills/ratatui/references/patterns.md
git commit -m "feat: add app architecture patterns reference"
```

---

## Task 7: Final verification

**Files:**
- All files in `.claude/skills/ratatui/`

**Depends on:** Tasks 2-6

- [ ] **Step 1: Verify file structure**

```bash
find .claude/skills/ratatui/ -type f | sort
```

Expected:
```
.claude/skills/ratatui/SKILL.md
.claude/skills/ratatui/references/custom-widgets.md
.claude/skills/ratatui/references/layout-advanced.md
.claude/skills/ratatui/references/patterns.md
.claude/skills/ratatui/references/widgets.md
```

- [ ] **Step 2: Verify SKILL.md line count is under 500**

```bash
wc -l .claude/skills/ratatui/SKILL.md
```

- [ ] **Step 3: Verify all internal links are valid**

Check that all `references/*.md` files referenced in SKILL.md exist.

- [ ] **Step 4: Spot-check code examples compile**

Create a temporary example file using the Quick Start code from SKILL.md. This avoids touching `src/main.rs`.

```bash
# Extract Quick Start example into a temporary example file
# Write to examples/verify_skill.rs
cargo check --example verify_skill
# If crossterm is needed as a direct dependency, temporarily add it to Cargo.toml
rm examples/verify_skill.rs
# Remove temporary crossterm dependency if added
```

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add .claude/skills/ratatui/
git commit -m "fix: address verification findings in ratatui skill"
```
