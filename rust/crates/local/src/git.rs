//! Running git from a command whose output is git's own.

use std::process::{Command, Stdio};

use crate::failure::Failure;

/// Runs git with `args`, its output passed on, failing with git's exit code.
pub(crate) fn git(args: &[&str]) -> Result<(), Failure> {
    let status = Command::new("git")
        .args(args)
        .status()
        .map_err(|error| Failure::new(format_args!("cannot run git: {error}"), 1))?;
    if status.success() {
        Ok(())
    } else {
        Err(Failure::passed_on(status))
    }
}

/// What git with `args` prints, its stderr passed on and its trailing newlines cut.
pub(crate) fn read(args: &[&str]) -> Result<String, Failure> {
    let output = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| Failure::new(format_args!("cannot run git: {error}"), 1))?;
    if !output.status.success() {
        return Err(Failure::passed_on(output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_owned())
}
