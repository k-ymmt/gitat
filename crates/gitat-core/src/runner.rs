use std::path::PathBuf;

use crate::GitError;

pub trait CommandRunner {
    fn run(&self, args: &[&str]) -> Result<String, GitError>;
}

pub struct ProcessRunner {
    repo_path: PathBuf,
}

impl ProcessRunner {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }
}
