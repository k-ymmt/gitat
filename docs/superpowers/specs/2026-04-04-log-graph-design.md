# Log Graph Visualization Design

## Overview

Add a visual commit graph to the Log tab, similar to `git log --graph`. The graph is computed from `CommitInfo.parent_hashes` and rendered with per-branch color coding to the left of each commit line.

## Style

- **Full graph**: branching/merging lines with `*`, `|`, `/`, `\` characters
- **Variable width**: graph characters immediately precede commit info (no fixed-width column)
- **Per-branch coloring**: each branch lane gets a distinct color from a rotating palette (lazygit style)

## Architecture

### New module: `gitat-core/src/graph.rs`

Computes graph layout from commit history. No UI dependencies.

**Input**: `&[CommitInfo]` (ordered newest-first, as returned by `get_log`)

**Output**: `Vec<GraphRow>`, one per commit, where each `GraphRow` contains a sequence of `GraphCell` values representing the graph characters for that line.

### Data Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct GraphCell {
    pub symbol: char,       // '*', '|', '/', '\', ' ', '_'
    pub color_index: usize, // Rotating index into color palette
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphRow {
    pub cells: Vec<GraphCell>,
}

pub fn build_graph(commits: &[CommitInfo]) -> Vec<GraphRow>;
```

### Graph Computation Algorithm

Maintains a list of "active lanes" — columns currently tracking a commit hash that hasn't been reached yet.

For each commit (top to bottom):

1. **Find lane**: locate the lane whose tracked hash matches this commit. If none exists, append a new lane.
2. **Place node**: put `*` in the commit's lane. All other active lanes get `|`.
3. **Assign parents**:
   - First parent: inherits the current lane (continues straight down).
   - Additional parents (merge): assigned to an existing lane tracking that hash, or a new lane is created.
4. **Connection lines**: when a merge parent is in a different column, insert `/` or `\` characters to connect them. For merges where branches converge, a connector row may be inserted between commit rows.
5. **Lane cleanup**: when a lane's tracked commit is reached and has no further use, remove it (shift remaining lanes left if possible).
6. **Color assignment**: each new lane gets `next_color_index`, which increments and wraps around the palette size.

### Color Palette

Add to `gitat-ui/src/theme.rs`:

```rust
const GRAPH_COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
];

pub fn graph_color(index: usize) -> Style {
    Style::default().fg(GRAPH_COLORS[index % GRAPH_COLORS.len()])
}
```

### Rendering Changes

In `gitat-ui/src/views/log.rs`, modify `render_log_list_items`:

1. Call `build_graph(&app.log_entries)` to get graph rows.
2. For each commit's `ListItem`, prepend `Span` elements from `GraphRow.cells` (each cell mapped to a colored `Span` via `Theme::graph_color(cell.color_index)`).
3. The "Uncommitted Changes" item at index 0 has no graph row — prepend appropriate continuation lines (`|` for each active lane) or leave blank.

### Handling the Uncommitted Changes Item

The "Uncommitted Changes" entry at index 0 is not a real commit and has no graph data. Options:
- Show continuation `|` lines for active lanes above the first commit, connecting to the first commit's node.
- If there are uncommitted changes (status non-empty), show a `*` node in the first lane to indicate working tree state.

Decision: Show `*` in the first lane if there are uncommitted changes, with `|` continuation to the first real commit below.

## Testing

Snapshot tests in `gitat-core/src/graph.rs` using `insta::assert_debug_snapshot!()`:

1. **Linear history**: 3-4 commits with single parents → straight `*`/`|` column
2. **Simple merge**: branch + merge back → fork and join lines
3. **Multiple parallel branches**: 2-3 concurrent branches → multiple columns
4. **Root commit**: single commit with no parents → lone `*`
5. **Octopus merge**: commit with 3+ parents (edge case)

## Files to Modify

- **New**: `crates/gitat-core/src/graph.rs` — graph computation module
- **Modify**: `crates/gitat-core/src/lib.rs` — add `pub mod graph;`
- **Modify**: `crates/gitat-ui/src/views/log.rs` — integrate graph into list rendering
- **Modify**: `crates/gitat-ui/src/theme.rs` — add graph color palette

## Scope Boundaries

- Graph is computed client-side from `parent_hashes`; no changes to git command execution.
- No interactive graph features (clicking on nodes, collapsing branches).
- Graph is display-only in the Log list; CommitDetail mode is unchanged.
