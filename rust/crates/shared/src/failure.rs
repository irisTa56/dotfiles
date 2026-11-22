//! How a command reports what ended its run early.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

/// What ends a run before it finishes: what it writes to stderr, line ends included,
/// which may be nothing, and the exit status.
pub struct Failure {
    stderr: Vec<u8>,
    code: u8,
}

impl Failure {
    #[must_use]
    pub fn new(message: fmt::Arguments<'_>, code: u8) -> Self {
        Failure {
            stderr: format!("{message}\n").into_bytes(),
            code,
        }
    }

    /// What ends a run with `code` where what failed has said why on stderr itself.
    #[must_use]
    pub fn quiet(code: u8) -> Self {
        Failure {
            stderr: Vec::new(),
            code,
        }
    }

    /// What ends a run with `code`, writing `stderr` byte for byte: what another program
    /// printed, say, followed by the lines that say what failed.
    #[must_use]
    pub fn verbatim(stderr: Vec<u8>, code: u8) -> Self {
        Failure { stderr, code }
    }

    pub fn report(self) -> ExitCode {
        let _ = io::stderr().write_all(&self.stderr);
        ExitCode::from(self.code)
    }
}
