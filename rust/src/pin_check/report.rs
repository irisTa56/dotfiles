//! How a run says what it could not do, on stderr.

use std::io;

use super::{Failure, Status};
use crate::git::Run;

impl Failure<'_> {
    pub(super) fn report(self) {
        eprintln!("[fail] {}", self.0);
    }

    /// git's own reason goes out above the line that says what could not be done.
    pub(super) fn report_git(self, run: &Run) -> Status {
        eprint!("{}", run.stderr());
        self.report();
        Status::Unknown
    }
}

impl Status {
    /// An error from the filesystem is no answer about the pins, so it ends the run as one.
    pub(super) fn stopped_on(error: &io::Error) -> Self {
        Failure(&format!("stopped on {error}")).report();
        Status::Unknown
    }
}
