//! How a run says what it could not do, on stderr, and how it ends.

use std::io;

use dotfiles_shared::Failure;

use super::{FailLine, Status};
use crate::git::Run;
use crate::stream::Stream;

impl FailLine<'_> {
    /// Writes the line, for a run that goes on.
    pub(super) fn report(self) {
        Stream::Stderr.line(format_args!("{}", self.text()));
    }

    /// Writes git's own reason and then the line, for a run that goes on.
    pub(super) fn report_git(self, run: &Run) {
        Stream::Stderr.bytes(&self.after_git(run));
    }

    /// What ends the run on this line, as one that could not tell.
    pub(super) fn ending(self) -> Failure {
        Failure::new(format_args!("{}", self.text()), Status::Unknown.code())
    }

    /// What ends the run on this line, as one that could not tell, with git's own reason
    /// above it.
    pub(super) fn ending_git(self, run: &Run) -> Failure {
        Failure::verbatim(self.after_git(run), Status::Unknown.code())
    }

    /// git's own reason, and the line below it that says what could not be done.
    pub(super) fn after_git(self, run: &Run) -> Vec<u8> {
        let mut bytes = run.stderr().as_bytes().to_vec();
        bytes.extend_from_slice(self.line().as_bytes());
        bytes
    }

    /// The line, with its line end.
    pub(super) fn line(self) -> String {
        format!("{}\n", self.text())
    }

    fn text(self) -> String {
        format!("[fail] {}", self.0)
    }
}

impl Status {
    /// An error from the filesystem is no answer about the pins, so it ends the run as one.
    pub(super) fn stopped_on(error: &io::Error) -> Failure {
        FailLine(&format!("stopped on {error}")).ending()
    }

    /// How a run that checked every pin ends: each pin has said what it found, so a status
    /// other than `Ok` ends the run with nothing more to say.
    pub(super) fn ended(self) -> Result<(), Failure> {
        match self {
            Status::Ok => Ok(()),
            _ => Err(Failure::quiet(self.code())),
        }
    }

    /// The status the run exits with.
    pub(super) fn code(self) -> u8 {
        self as u8
    }
}
