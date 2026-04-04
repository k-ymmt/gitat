pub mod branch;
pub mod commit;
pub mod commit_detail;
pub mod conflict;
pub mod diff;
pub mod graph;
pub mod log;
pub mod remote;
pub mod runner;
pub mod stage;
pub mod status;

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum GitError {
    #[error("git command failed: {command} (exit code {exit_code})\n{stderr}")]
    CommandFailed {
        command: String,
        stderr: String,
        exit_code: i32,
    },
    #[error("failed to parse git output: {0}")]
    ParseError(String),
    #[error("not a git repository")]
    NotARepository,
    #[error("IO error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::IoError(e.to_string())
    }
}
