use crate::GitError;
use crate::runner::CommandRunner;

pub fn stage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["add", "--", path])?;
    Ok(())
}

pub fn unstage_file(runner: &dyn CommandRunner, path: &str) -> Result<(), GitError> {
    runner.run(&["reset", "HEAD", "--", path])?;
    Ok(())
}
