use crate::log::CommitInfo;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphCell {
    pub symbol: char,
    pub color_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphRow {
    pub cells: Vec<GraphCell>,
}

pub fn build_graph(commits: &[CommitInfo]) -> Vec<GraphRow> {
    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut lane_colors: Vec<usize> = Vec::new();
    let mut next_color: usize = 0;
    let mut rows = Vec::new();

    for commit in commits {
        let commit_lane = match lanes.iter().position(|l| l.as_deref() == Some(&*commit.hash)) {
            Some(pos) => pos,
            None => {
                match lanes.iter().position(|l| l.is_none()) {
                    Some(pos) => {
                        lanes[pos] = Some(commit.hash.clone());
                        lane_colors[pos] = next_color;
                        next_color += 1;
                        pos
                    }
                    None => {
                        lanes.push(Some(commit.hash.clone()));
                        lane_colors.push(next_color);
                        next_color += 1;
                        lanes.len() - 1
                    }
                }
            }
        };

        let commit_color = lane_colors[commit_lane];

        let mut cells = Vec::new();
        for (i, lane) in lanes.iter().enumerate() {
            if i == commit_lane {
                cells.push(GraphCell {
                    symbol: '*',
                    color_index: commit_color,
                });
            } else if lane.is_some() {
                cells.push(GraphCell {
                    symbol: '|',
                    color_index: lane_colors[i],
                });
            } else {
                cells.push(GraphCell {
                    symbol: ' ',
                    color_index: 0,
                });
            }
        }

        if commit.parent_hashes.is_empty() {
            lanes[commit_lane] = None;
        } else {
            lanes[commit_lane] = Some(commit.parent_hashes[0].clone());
            for parent in &commit.parent_hashes[1..] {
                let already_tracked =
                    lanes.iter().any(|l| l.as_deref() == Some(parent.as_str()));
                if !already_tracked {
                    match lanes.iter().position(|l| l.is_none()) {
                        Some(pos) => {
                            lanes[pos] = Some(parent.clone());
                            lane_colors[pos] = next_color;
                            next_color += 1;
                        }
                        None => {
                            lanes.push(Some(parent.clone()));
                            lane_colors.push(next_color);
                            next_color += 1;
                        }
                    }
                }
            }
        }

        let mut i = 0;
        while i < lanes.len() {
            if let Some(ref hash) = lanes[i] {
                let earlier = lanes[..i]
                    .iter()
                    .position(|l| l.as_deref() == Some(hash.as_str()));
                if earlier.is_some() {
                    lanes[i] = None;
                }
            }
            i += 1;
        }

        while lanes.last() == Some(&None) {
            lanes.pop();
            lane_colors.pop();
        }

        rows.push(GraphRow { cells });
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::CommitInfo;

    fn commit(hash: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            hash: hash.to_string(),
            short_hash: hash[..3.min(hash.len())].to_string(),
            author: "Test".to_string(),
            date: "2026-01-01".to_string(),
            message: format!("commit {hash}"),
            refs: vec![],
            parent_hashes: parents.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_linear_history() {
        let commits = vec![
            commit("aaa", &["bbb"]),
            commit("bbb", &["ccc"]),
            commit("ccc", &[]),
        ];
        let graph = build_graph(&commits);
        assert_eq!(graph.len(), 3);
        for row in &graph {
            assert_eq!(row.cells.len(), 1);
            assert_eq!(row.cells[0].symbol, '*');
        }
        assert_eq!(graph[0].cells[0].color_index, graph[1].cells[0].color_index);
        assert_eq!(graph[1].cells[0].color_index, graph[2].cells[0].color_index);
    }

    #[test]
    fn test_root_commit() {
        let commits = vec![commit("aaa", &[])];
        let graph = build_graph(&commits);
        assert_eq!(graph.len(), 1);
        assert_eq!(graph[0].cells.len(), 1);
        assert_eq!(graph[0].cells[0].symbol, '*');
    }

    #[test]
    fn snapshot_linear_history() {
        let commits = vec![
            commit("aaa", &["bbb"]),
            commit("bbb", &["ccc"]),
            commit("ccc", &[]),
        ];
        let graph = build_graph(&commits);
        insta::assert_debug_snapshot!(graph);
    }
}
