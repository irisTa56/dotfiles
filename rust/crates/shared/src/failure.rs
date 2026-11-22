//! How a command reports what ended its run early.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

/// What ends a run before it finishes: a message for stderr, where one is due, and the
/// exit status.
pub struct Failure {
    message: Option<String>,
    code: u8,
}

impl Failure {
    #[must_use]
    pub fn new(message: fmt::Arguments<'_>, code: u8) -> Self {
        Failure {
            message: Some(message.to_string()),
            code,
        }
    }

    /// What ends a run with `code` where what failed has said why on stderr itself.
    #[must_use]
    pub fn quiet(code: u8) -> Self {
        Failure {
            message: None,
            code,
        }
    }

    pub fn report(self) -> ExitCode {
        if let Some(message) = self.message {
            let _ = writeln!(io::stderr(), "{message}");
        }
        ExitCode::from(self.code)
    }
}
