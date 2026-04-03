use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub refs: Vec<String>,
    pub parent_hashes: Vec<String>,
}

/// Format string for git log: hash, short hash, parents, refs, author, date, subject
/// Fields separated by \x1f (unit separator), records separated by \x1e (record separator)
const LOG_FORMAT: &str = "%H\x1f%h\x1f%P\x1f%D\x1f%an\x1f%ai\x1f%s\x1e";

pub fn parse_log(output: &str) -> Result<Vec<CommitInfo>, GitError> {
    let mut commits = Vec::new();

    for record in output.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }

        let fields: Vec<&str> = record.split('\x1f').collect();
        if fields.len() < 7 {
            return Err(GitError::ParseError(format!(
                "expected 7 fields, got {}: {record}",
                fields.len()
            )));
        }

        let parent_hashes = if fields[2].is_empty() {
            Vec::new()
        } else {
            fields[2].split(' ').map(|s| s.to_string()).collect()
        };

        let refs = if fields[3].is_empty() {
            Vec::new()
        } else {
            fields[3].split(", ").map(|s| s.to_string()).collect()
        };

        commits.push(CommitInfo {
            hash: fields[0].to_string(),
            short_hash: fields[1].to_string(),
            parent_hashes,
            refs,
            author: fields[4].to_string(),
            date: fields[5].to_string(),
            message: fields[6].to_string(),
        });
    }

    Ok(commits)
}

pub fn get_log(runner: &dyn CommandRunner, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
    let output = runner.run(&[
        "log",
        &format!("--max-count={limit}"),
        &format!("--format={LOG_FORMAT}"),
    ])?;
    parse_log(&output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_log() {
        let result = parse_log("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_single_commit() {
        let input = "abc123def456\x1fabc123d\x1f\x1fHEAD -> main\x1fJohn Doe\x1f2026-04-03 10:00:00 +0900\x1ffeat: initial commit\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hash, "abc123def456");
        assert_eq!(result[0].short_hash, "abc123d");
        assert_eq!(result[0].author, "John Doe");
        assert_eq!(result[0].message, "feat: initial commit");
        assert_eq!(result[0].refs, vec!["HEAD -> main"]);
        assert!(result[0].parent_hashes.is_empty());
    }

    #[test]
    fn test_parse_commit_with_parents() {
        let input = "abc123\x1fabc\x1fdef456 ghi789\x1f\x1fAlice\x1f2026-04-03\x1fmerge\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result[0].parent_hashes, vec!["def456", "ghi789"]);
    }

    #[test]
    fn test_parse_multiple_commits() {
        let input = "aaa\x1fa\x1f\x1fHEAD\x1fBob\x1f2026-04-03\x1ffirst\x1ebbb\x1fb\x1faaa\x1f\x1fBob\x1f2026-04-02\x1fsecond\x1e";
        let result = parse_log(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].message, "first");
        assert_eq!(result[1].message, "second");
    }
}
