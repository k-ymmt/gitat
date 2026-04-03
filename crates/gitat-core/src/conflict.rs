use std::path::Path;

use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictRegion {
    Clean(Vec<String>),
    Conflict {
        ours: Vec<String>,
        theirs: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictFile {
    pub path: String,
    pub regions: Vec<ConflictRegion>,
}

pub fn parse_conflict_markers(path: &str, content: &str) -> Result<ConflictFile, GitError> {
    let mut regions = Vec::new();
    let mut clean_lines = Vec::new();
    let mut ours_lines: Option<Vec<String>> = None;
    let mut theirs_lines: Option<Vec<String>> = None;
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            if !clean_lines.is_empty() {
                regions.push(ConflictRegion::Clean(std::mem::take(&mut clean_lines)));
            }
            in_ours = true;
            ours_lines = Some(Vec::new());
        } else if line.starts_with("=======") && in_ours {
            in_ours = false;
            in_theirs = true;
            theirs_lines = Some(Vec::new());
        } else if line.starts_with(">>>>>>>") && in_theirs {
            in_theirs = false;
            if let (Some(ours), Some(theirs)) = (ours_lines.take(), theirs_lines.take()) {
                regions.push(ConflictRegion::Conflict { ours, theirs });
            }
        } else if in_ours {
            if let Some(ref mut lines) = ours_lines {
                lines.push(line.to_string());
            }
        } else if in_theirs {
            if let Some(ref mut lines) = theirs_lines {
                lines.push(line.to_string());
            }
        } else {
            clean_lines.push(line.to_string());
        }
    }

    if !clean_lines.is_empty() {
        regions.push(ConflictRegion::Clean(clean_lines));
    }

    Ok(ConflictFile {
        path: path.to_string(),
        regions,
    })
}

pub fn get_conflict_paths(runner: &dyn CommandRunner) -> Result<Vec<String>, GitError> {
    let output = runner.run(&["diff", "--name-only", "--diff-filter=U"])?;
    Ok(output.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
}

pub fn resolve_file(runner: &dyn CommandRunner, path: &str, content: &str) -> Result<(), GitError> {
    std::fs::write(Path::new(path), content)
        .map_err(|e| GitError::IoError(e.to_string()))?;
    runner.run(&["add", "--", path])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflicts() {
        let content = "line1\nline2\nline3\n";
        let result = parse_conflict_markers("file.rs", content).unwrap();
        assert_eq!(result.regions.len(), 1);
        match &result.regions[0] {
            ConflictRegion::Clean(lines) => assert_eq!(lines.len(), 3),
            _ => panic!("expected Clean region"),
        }
    }

    #[test]
    fn test_single_conflict() {
        let content = "\
line1
<<<<<<< HEAD
ours line
=======
theirs line
>>>>>>> feature
line2
";
        let result = parse_conflict_markers("file.rs", content).unwrap();
        assert_eq!(result.regions.len(), 3);
        match &result.regions[0] {
            ConflictRegion::Clean(lines) => assert_eq!(lines, &["line1"]),
            _ => panic!("expected Clean"),
        }
        match &result.regions[1] {
            ConflictRegion::Conflict { ours, theirs } => {
                assert_eq!(ours, &["ours line"]);
                assert_eq!(theirs, &["theirs line"]);
            }
            _ => panic!("expected Conflict"),
        }
        match &result.regions[2] {
            ConflictRegion::Clean(lines) => assert_eq!(lines, &["line2"]),
            _ => panic!("expected Clean"),
        }
    }

    #[test]
    fn test_multiple_conflicts() {
        let content = "\
<<<<<<< HEAD
a
=======
b
>>>>>>> feat
middle
<<<<<<< HEAD
c
=======
d
>>>>>>> feat
";
        let result = parse_conflict_markers("f.rs", content).unwrap();
        assert_eq!(result.regions.len(), 3);
    }

    #[test]
    fn snapshot_no_conflicts() {
        let content = "line1\nline2\nline3\n";
        let result = parse_conflict_markers("clean.rs", content).unwrap();
        insta::assert_debug_snapshot!(result);
    }

    #[test]
    fn snapshot_single_conflict() {
        let content = "\
line1
<<<<<<< HEAD
our change
our change 2
=======
their change
>>>>>>> feature
line3
";
        let result = parse_conflict_markers("file.rs", content).unwrap();
        insta::assert_debug_snapshot!(result);
    }

    #[test]
    fn snapshot_multiple_conflicts() {
        let content = "\
header
<<<<<<< HEAD
a1
a2
=======
b1
>>>>>>> feat
middle
<<<<<<< HEAD
c1
=======
d1
d2
>>>>>>> feat
footer
";
        let result = parse_conflict_markers("multi.rs", content).unwrap();
        insta::assert_debug_snapshot!(result);
    }
}
