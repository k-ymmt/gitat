use crate::runner::CommandRunner;
use crate::GitError;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Unmodified,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusEntry {
    pub path: String,
    pub index_status: FileStatus,
    pub worktree_status: FileStatus,
}

fn parse_file_status(c: char) -> FileStatus {
    match c {
        'M' => FileStatus::Modified,
        'A' => FileStatus::Added,
        'D' => FileStatus::Deleted,
        'R' => FileStatus::Renamed,
        'C' => FileStatus::Copied,
        '?' => FileStatus::Untracked,
        ' ' => FileStatus::Unmodified,
        _ => FileStatus::Unmodified,
    }
}

pub fn parse_status(output: &str) -> Result<Vec<StatusEntry>, GitError> {
    let mut entries = Vec::new();

    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let index_status = parse_file_status(chars[0]);
        let worktree_status = parse_file_status(chars[1]);
        let path = line[3..].to_string();

        entries.push(StatusEntry {
            path,
            index_status,
            worktree_status,
        });
    }

    Ok(entries)
}

pub fn get_status(runner: &dyn CommandRunner) -> Result<Vec<StatusEntry>, GitError> {
    let output = runner.run(&["status", "--porcelain=v1"])?;
    parse_status(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_status() {
        let result = parse_status("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_modified_file() {
        let result = parse_status(" M src/main.rs\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/main.rs");
        assert_eq!(result[0].index_status, FileStatus::Unmodified);
        assert_eq!(result[0].worktree_status, FileStatus::Modified);
    }

    #[test]
    fn test_parse_staged_and_modified() {
        let result = parse_status("MM src/lib.rs\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Modified);
        assert_eq!(result[0].worktree_status, FileStatus::Modified);
    }

    #[test]
    fn test_parse_added_file() {
        let result = parse_status("A  new_file.rs\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Added);
        assert_eq!(result[0].worktree_status, FileStatus::Unmodified);
    }

    #[test]
    fn test_parse_untracked() {
        let result = parse_status("?? untracked.txt\n").unwrap();
        assert_eq!(result[0].index_status, FileStatus::Untracked);
        assert_eq!(result[0].worktree_status, FileStatus::Untracked);
    }

    #[test]
    fn test_parse_multiple_entries() {
        let input = " M src/main.rs\nA  new.rs\n?? notes.txt\n";
        let result = parse_status(input).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn snapshot_mixed_status() {
        let input = "\
MM src/lib.rs
A  new_file.rs
 D deleted.rs
 M src/main.rs
?? untracked.txt
R  old_name.rs
";
        let result = parse_status(input).unwrap();
        insta::assert_debug_snapshot!(result);
    }
}
