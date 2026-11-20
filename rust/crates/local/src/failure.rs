//! How a command reports what ended its run early.

use std::fmt;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitCode, ExitStatus};

/// What ends a run before it finishes: a message for stderr, where one is due, and the
/// exit status.
pub(crate) struct Failure {
    message: Option<String>,
    code: u8,
}

impl Failure {
    pub(crate) fn new(message: fmt::Arguments<'_>, code: u8) -> Self {
        Failure {
            message: Some(message.to_string()),
            code,
        }
    }

    /// What ends a run where a command it ran failed with `status`, having said why on
    /// stderr itself.
    pub(crate) fn passed_on(status: ExitStatus) -> Self {
        Failure {
            message: None,
            code: exit_code(status),
        }
    }

    /// What ends a run whose output cannot be written.
    pub(crate) fn writing(error: &io::Error) -> Self {
        Failure::new(format_args!("cannot write the output: {error}"), 1)
    }

    pub(crate) fn report(self) -> ExitCode {
        if let Some(message) = self.message {
            let _ = writeln!(io::stderr(), "{message}");
        }
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
