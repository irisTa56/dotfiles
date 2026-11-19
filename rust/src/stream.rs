//! The streams a run writes to, and what a write does where no one reads it any longer.
//!
//! Rust's runtime ignores SIGPIPE, so `println!` and `eprintln!` panic once the reader of
//! a pipe has gone, as `| head` leaves it. Every task reports its outcome by its exit
//! status, and a reader that stopped reading wants no more, so a write that fails is
//! dropped rather than ending the run.

use std::fmt;
use std::io::{self, Write};

/// One of the process's output streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    /// Where a task writes what a caller may read on, such as a pin that matches.
    Stdout,
    /// Where a task reports what fails, and passes on what git said.
    Stderr,
}

impl Stream {
    /// Writes `line` and a line end, dropping the write where it fails.
    pub(crate) fn line(self, line: fmt::Arguments<'_>) {
        let _ = match self {
            Stream::Stdout => writeln!(io::stdout(), "{line}"),
            Stream::Stderr => writeln!(io::stderr(), "{line}"),
        };
    }

    /// Writes `bytes` as they are, dropping the write where it fails.
    pub(crate) fn bytes(self, bytes: &[u8]) {
        let _ = match self {
            Stream::Stdout => io::stdout().write_all(bytes),
            Stream::Stderr => io::stderr().write_all(bytes),
        };
    }
}
