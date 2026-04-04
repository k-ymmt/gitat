use crate::diff::{self, DiffFile};
use crate::runner::CommandRunner;
use crate::GitError;

#[derive(Debug, Clone, PartialEq)]
pub enum FileChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitFileEntry {
    pub path: String,
    pub status: FileChangeStatus,
}

pub fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, GitError> {
    let mut entries = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let status_str = parts[0];
        let (status, path) = if status_str.starts_with('R') {
            let new_path = if parts.len() >= 3 { parts[2] } else { parts[1] };
            (FileChangeStatus::Renamed, new_path.to_string())
        } else {
            let status = match status_str {
                "A" => FileChangeStatus::Added,
                "D" => FileChangeStatus::Deleted,
                _ => FileChangeStatus::Modified,
            };
            (status, parts[1].to_string())
        };
        entries.push(CommitFileEntry { path, status });
    }
    Ok(entries)
}

pub fn get_commit_files(
    runner: &dyn CommandRunner,
    hash: &str,
    first_parent: Option<&str>,
) -> Result<Vec<CommitFileEntry>, GitError> {
    let output = if let Some(parent) = first_parent {
        runner.run(&["diff-tree", "--no-commit-id", "-r", "--name-status", parent, hash])?
    } else {
        runner.run(&["diff-tree", "--no-commit-id", "-r", "--name-status", hash])?
    };
    parse_commit_files(&output)
}

pub fn get_commit_file_diff(
    runner: &dyn CommandRunner,
    hash: &str,
    parent_hash: Option<&str>,
    file_path: &str,
) -> Result<Vec<DiffFile>, GitError> {
    let output = if let Some(parent) = parent_hash {
        let range = format!("{parent}..{hash}");
        runner.run(&["diff", &range, "--", file_path])?
    } else {
        runner.run(&["show", "--format=", hash, "--", file_path])?
    };
    diff::parse_diff(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_commit_files("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_modified_file() {
        let input = "M\tsrc/main.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[0].status, FileChangeStatus::Modified);
    }

    #[test]
    fn test_parse_multiple_files() {
        let input = "A\tsrc/new.rs\nM\tsrc/main.rs\nD\tsrc/old.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].status, FileChangeStatus::Added);
        assert_eq!(result[0].path, "src/new.rs");
        assert_eq!(result[1].status, FileChangeStatus::Modified);
        assert_eq!(result[1].path, "src/main.rs");
        assert_eq!(result[2].status, FileChangeStatus::Deleted);
        assert_eq!(result[2].path, "src/old.rs");
    }

    #[test]
    fn test_parse_renamed_file() {
        let input = "R100\told_name.rs\tnew_name.rs\n";
        let result = parse_commit_files(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, FileChangeStatus::Renamed);
        assert_eq!(result[0].path, "new_name.rs");
    }

    #[test]
    fn snapshot_multiple_files() {
        let input = "A\tsrc/new.rs\nM\tsrc/main.rs\nD\tsrc/old.rs\n";
        let result = parse_commit_files(input).unwrap();
        insta::assert_debug_snapshot!(result);
    }

    #[test]
    fn test_get_commit_files() {
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status abc123",
                "M\tsrc/main.rs\nA\tsrc/new.rs\n",
            );
        let result = get_commit_files(&runner, "abc123", None).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[1].path, "src/new.rs");
    }

    #[test]
    fn test_get_commit_files_merge_commit() {
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "diff-tree --no-commit-id -r --name-status parent1 merge123",
                "M\tsrc/main.rs\nA\tsrc/new.rs\n",
            );
        let result = get_commit_files(&runner, "merge123", Some("parent1")).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[1].path, "src/new.rs");
    }

    #[test]
    fn test_get_commit_file_diff_with_parent() {
        let diff_output = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
-    old();
+    new();
+    extra();
 }
";
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "diff parent123..abc123 -- src/main.rs",
                diff_output,
            );
        let result = get_commit_file_diff(&runner, "abc123", Some("parent123"), "src/main.rs").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "src/main.rs");
    }

    #[test]
    fn test_get_commit_file_diff_root_commit() {
        let diff_output = "\
diff --git a/src/main.rs b/src/main.rs
new file mode 100644
--- /dev/null
+++ b/src/main.rs
@@ -0,0 +1,2 @@
+fn main() {}
+fn helper() {}
";
        let runner = crate::runner::MockRunner::new()
            .with_response(
                "show --format= abc123 -- src/main.rs",
                diff_output,
            );
        let result = get_commit_file_diff(&runner, "abc123", None, "src/main.rs").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].old_path, "/dev/null");
    }
}
