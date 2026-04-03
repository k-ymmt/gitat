use crate::GitError;
use crate::runner::CommandRunner;

#[derive(Debug, Clone, PartialEq)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub short_hash: String,
    pub last_commit: String,
}

pub fn parse_branches(output: &str) -> Result<Vec<BranchInfo>, GitError> {
    let mut branches = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let is_current = line.starts_with('*');
        let line = line.trim_start_matches(['*', ' ']);
        if line.contains(" -> ") { continue; } // Skip symbolic refs
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            branches.push(BranchInfo {
                name: parts[0].to_string(),
                is_current,
                upstream: None,
                short_hash: parts[1].to_string(),
                last_commit: parts.get(2).unwrap_or(&"").to_string(),
            });
        }
    }
    Ok(branches)
}

pub fn current_branch(runner: &dyn CommandRunner) -> Result<String, GitError> {
    let output = runner.run(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    Ok(output.trim().to_string())
}

pub fn list_branches(runner: &dyn CommandRunner) -> Result<Vec<BranchInfo>, GitError> {
    let output = runner.run(&["branch", "-v", "--no-color"])?;
    parse_branches(&output)
}

pub fn create_branch(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["branch", name])?;
    Ok(())
}

pub fn checkout(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["checkout", name])?;
    Ok(())
}

pub fn delete_branch(runner: &dyn CommandRunner, name: &str) -> Result<(), GitError> {
    runner.run(&["branch", "-d", name])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branches() {
        let input = "* main       abc1234 latest commit\n  feature    def5678 wip feature\n";
        let result = parse_branches(input).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_current);
        assert_eq!(result[0].name, "main");
        assert!(!result[1].is_current);
        assert_eq!(result[1].name, "feature");
    }

    #[test]
    fn test_parse_empty_branches() {
        let result = parse_branches("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn snapshot_branches_with_current() {
        let input = "\
* main       abc1234 latest commit message
  feature    def5678 wip: add feature
  bugfix     ghi9012 fix: resolve crash
";
        let result = parse_branches(input).unwrap();
        insta::assert_debug_snapshot!(result);
    }

    #[test]
    fn snapshot_branches_with_symbolic_ref() {
        let input = "\
* main           abc1234 latest commit
  origin/HEAD -> origin/main
  feature        def5678 wip feature
";
        let result = parse_branches(input).unwrap();
        insta::assert_debug_snapshot!(result);
    }
}
