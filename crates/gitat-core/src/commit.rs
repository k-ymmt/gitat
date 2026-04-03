use crate::GitError;
use crate::runner::CommandRunner;

pub fn commit(runner: &dyn CommandRunner, message: &str) -> Result<(), GitError> {
    runner.run(&["commit", "-m", message])?;
    Ok(())
}

pub fn amend(runner: &dyn CommandRunner, message: &str) -> Result<(), GitError> {
    runner.run(&["commit", "--amend", "-m", message])?;
    Ok(())
}
