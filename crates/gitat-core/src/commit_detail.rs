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
}
