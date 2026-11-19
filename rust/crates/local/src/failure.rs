//! How a command reports what ended its run early.

use std::fmt;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitCode, ExitStatus};

/// What ends a run before it finishes: a message for stderr and the exit status.
pub(crate) struct Failure {
    message: String,
    code: u8,
}

impl Failure {
    pub(crate) fn new(message: fmt::Arguments<'_>, code: u8) -> Self {
        Failure {
            message: message.to_string(),
            code,
        }
    }

    pub(crate) fn report(self) -> ExitCode {
        let _ = writeln!(io::stderr(), "{}", self.message);
        ExitCode::from(self.code)
    }
}

/// The status a command ended with, a signal giving the one a shell gives.
pub(crate) fn exit_code(status: ExitStatus) -> u8 {
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1)
}
