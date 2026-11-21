//! How a command reports what ended its run early.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

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

    /// What ends a run with `code` where what failed has said why on stderr itself.
    pub(crate) fn quiet(code: u8) -> Self {
        Failure {
            message: None,
            code,
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
