use crate::GitError;
use crate::runner::CommandRunner;

pub fn push(runner: &dyn CommandRunner, remote: &str, branch: &str) -> Result<(), GitError> {
    runner.run(&["push", remote, branch])?;
    Ok(())
}

pub fn pull(runner: &dyn CommandRunner, remote: &str, branch: &str) -> Result<(), GitError> {
    runner.run(&["pull", remote, branch])?;
    Ok(())
}

pub fn fetch(runner: &dyn CommandRunner) -> Result<(), GitError> {
    runner.run(&["fetch", "--all"])?;
    Ok(())
}
