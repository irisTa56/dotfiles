//! A call to mise.

use std::fmt;
use std::io::{self, Read, Write};
use std::process::{ExitStatus, Stdio};

use crate::program::Program;
use dotfiles_shared::Failure;

/// mise, called with a list of arguments.
pub(super) struct Mise {
    args: Vec<String>,
}

impl Mise {
    pub(super) fn new(args: Vec<String>) -> Self {
        Mise { args }
    }

    /// What mise prints on stdout, given no input, or `None` where it fails. Its stderr
    /// is left to the terminal: its warnings would break the JSON read from stdout, and
    /// its errors are what the user needs to see.
    pub(super) fn output(&self) -> Result<Option<Vec<u8>>, Failure> {
        let output = Program::new("mise")
            .args(&self.args)
            .stderr(Stdio::inherit())
            .output()?;
        Ok(output.status.success().then_some(output.stdout))
    }

    /// Runs mise, given no input, passing on what it prints on stdout and leaving its
    /// stderr to the terminal.
    ///
    /// Its stdout reaches this one through a pipe rather than as this one itself, so
    /// mise writes as it does for a reader that is not a terminal.
    pub(super) fn pass_on(&self) -> Result<ExitStatus, Failure> {
        let mut child = Program::new("mise")
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let copied = match child.stdout.take() {
            Some(mut reader) => copy_to_stdout(&mut reader),
            None => Ok(()),
        };
        child
            .wait()
            .and_then(|status| copied.map(|()| status))
            .map_err(|error| Failure::new(format_args!("{self} failed: {error}"), 1))
    }
}

impl fmt::Display for Mise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mise {}", self.args.join(" "))
    }
}

/// Copies what `reader` yields to stdout until it ends.
fn copy_to_stdout(reader: &mut impl Read) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    io::copy(reader, &mut stdout)?;
    stdout.flush()
}
