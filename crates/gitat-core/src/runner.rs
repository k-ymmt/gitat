use std::path::PathBuf;

use crate::GitError;

pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
    fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, GitError>;
}

pub struct ProcessRunner {
    repo_path: PathBuf,
}

impl ProcessRunner {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }
}

impl CommandRunner for ProcessRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| GitError::IoError(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
    }

    fn run_with_stdin(&self, args: &[&str], stdin_data: &str) -> Result<String, GitError> {
        use std::io::Write;
        use std::process::Stdio;

        let mut child = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::IoError(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(stdin_data.as_bytes())
                .map_err(|e| GitError::IoError(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| GitError::IoError(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
    }
}

pub struct MockRunner {
    responses: std::collections::HashMap<String, Result<String, GitError>>,
}

impl MockRunner {
    pub fn new() -> Self {
        Self {
            responses: std::collections::HashMap::new(),
        }
    }

    pub fn with_response(mut self, key: &str, output: &str) -> Self {
        self.responses
            .insert(key.to_string(), Ok(output.to_string()));
        self
    }
}

impl Default for MockRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunner for MockRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let key = args.join(" ");
        self.responses.get(&key).cloned().unwrap_or_else(|| {
            Err(GitError::CommandFailed {
                command: format!("git {}", key),
                stderr: "mock: no response configured".to_string(),
                exit_code: 1,
            })
        })
    }

    fn run_with_stdin(&self, args: &[&str], _stdin_data: &str) -> Result<String, GitError> {
        self.run(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_runner_executes_git_version() {
        let runner = ProcessRunner::new(PathBuf::from("."));
        let result = runner.run(&["version"]);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("git version"));
    }

    #[test]
    fn test_process_runner_returns_error_for_invalid_command() {
        let runner = ProcessRunner::new(PathBuf::from("."));
        let result = runner.run(&["not-a-real-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_runner_run_with_stdin() {
        let runner = MockRunner::new().with_response("apply --cached", "");
        let result = runner.run_with_stdin(&["apply", "--cached"], "patch content");
        assert!(result.is_ok());
    }
}
