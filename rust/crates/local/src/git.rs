//! Running git.

use dotfiles_shared::Failure;

use crate::program::Program;

/// git with `args`, to run.
pub(crate) fn command(args: &[&str]) -> Program {
    Program::new("git").args(args)
}

/// Runs git with `args`, its output passed on, failing with git's exit code.
pub(crate) fn git(args: &[&str]) -> Result<(), Failure> {
    command(args).run()
}

/// What git with `args` prints, its stderr passed on and its trailing newlines cut.
pub(crate) fn read(args: &[&str]) -> Result<String, Failure> {
    let stdout = command(args).read()?;
    Ok(String::from_utf8_lossy(&stdout)
        .trim_end_matches('\n')
        .to_owned())
}
