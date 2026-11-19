//! A call to mise.

use std::fmt;
use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};

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
    pub(super) fn output(&self) -> io::Result<Option<Vec<u8>>> {
        let output = Command::new("mise")
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
    pub(super) fn pass_on(&self) -> io::Result<ExitStatus> {
        let mut child = Command::new("mise")
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let copied = match child.stdout.take() {
            Some(mut reader) => copy_to_stdout(&mut reader),
            None => Ok(()),
        };
        let status = child.wait()?;
        copied.map(|()| status)
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
