//! How a run says what it could not do, on stderr.

use super::Status;
use crate::git::Run;

pub(super) fn fail(message: &str) {
    eprintln!("[fail] {message}");
}

/// git's own reason goes out above the line that says what could not be done.
pub(super) fn fail_git(run: &Run, message: &str) -> Status {
    eprint!("{}", run.stderr());
    fail(message);
    Status::Unknown
}
