pub mod runner;
pub mod status;
pub mod log;
pub mod diff;
pub mod branch;
pub mod stage;
pub mod commit;
pub mod remote;

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
