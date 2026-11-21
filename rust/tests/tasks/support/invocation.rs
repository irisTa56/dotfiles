//! A run of the binary, a wrapper or a shell task in a checkout, put together one setting at
//! a time, and how the run ended.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A run not yet started: the command, the checkout it runs in, what it is fed on stdin,
/// and which of its output streams, if any, is closed.
#[must_use = "an invocation runs nothing until `run` is called"]
pub(crate) struct Invocation {
    command: Command,
    root: PathBuf,
    input: Vec<u8>,
    closed: Option<Stream>,
}

/// One of a run's output streams.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stream {
    Stdout,
    Stderr,
}

/// How a run ended: its exit code, and stdout and stderr as one stream.
pub(crate) struct Run {
    code: i32,
    output: String,
}

impl Invocation {
    /// A run of `command`, which already runs in the checkout at `root`, fed nothing and
    /// with both streams read.
    pub(super) fn new(command: Command, root: PathBuf) -> Self {
        Invocation {
            command,
            root,
            input: Vec::new(),
            closed: None,
        }
    }

    /// Passes `arg` after the arguments given so far.
    pub(crate) fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.command.arg(arg);
        self
    }

    /// Passes each of `args` after the arguments given so far.
    pub(crate) fn args<A: AsRef<OsStr>>(mut self, args: impl IntoIterator<Item = A>) -> Self {
        self.command.args(args);
        self
    }

    /// Sets the variable `name` to `value` over the run's environment.
    pub(crate) fn env(mut self, name: &str, value: impl AsRef<OsStr>) -> Self {
        self.command.env(name, value);
        self
    }

    /// Removes the variable `name` from the run's environment.
    pub(crate) fn env_remove(mut self, name: &str) -> Self {
        self.command.env_remove(name);
        self
    }

    /// Runs in the directory `dir` of the checkout rather than at its top.
    pub(crate) fn in_dir(mut self, dir: &str) -> Self {
        self.command.current_dir(self.root.join(dir));
        self
    }

    /// Feeds `input` on stdin, which need not be UTF-8.
    pub(crate) fn input(mut self, input: impl AsRef<[u8]>) -> Self {
        self.input = input.as_ref().to_vec();
        self
    }

    /// Makes `stream` a pipe whose reading end is already closed, so that the run's output
    /// is the other stream alone.
    pub(crate) fn closing(mut self, stream: Stream) -> Self {
        self.closed = Some(stream);
        self
    }

    /// Runs the command to its end, feeding it its input and reading its output.
    pub(crate) fn run(mut self) -> Run {
        let mut reader = self.connect_output();
        let Invocation {
            mut command, input, ..
        } = self;
        let mut child = command
            .stdin(Stdio::piped())
            .spawn()
            .expect("the command runs");
        // A command that exits without reading its input fails the write, which says
        // nothing about the run.
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let _ = stdin.write_all(&input);
        drop(stdin);
        // The command holds the pipes' writing ends until it is dropped, and reading
        // waits for every one to close.
        drop(command);
        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .expect("the output is UTF-8");
        let status = child.wait().expect("the command's run ends");
        Run {
            code: status
                .code()
                .expect("the command exits rather than being killed"),
            output,
        }
    }

    /// Points stdout and stderr at one pipe, or, where a stream is to be closed, that one at
    /// a pipe whose reading end is already closed and the other at the pipe, and returns
    /// the pipe's reading end.
    fn connect_output(&mut self) -> io::PipeReader {
        let (reader, writer) = io::pipe().expect("a pipe can be made");
        match self.closed {
            None => {
                self.command
                    .stdout(writer.try_clone().expect("the pipe can be shared"))
                    .stderr(writer);
            }
            Some(stream) => {
                let (closed_reader, closed_writer) = io::pipe().expect("a pipe can be made");
                drop(closed_reader);
                match stream {
                    Stream::Stdout => self.command.stdout(closed_writer).stderr(writer),
                    Stream::Stderr => self.command.stdout(writer).stderr(closed_writer),
                };
            }
        }
        reader
    }
}

impl Run {
    /// The code the run exited with.
    pub(crate) fn code(&self) -> i32 {
        self.code
    }

    /// What the run printed: stdout and stderr as one stream, or the stream left open.
    pub(crate) fn output(&self) -> &str {
        &self.output
    }

    /// Asserts that the run exited with `code` and printed each of `parts`.
    pub(crate) fn expect(&self, code: i32, parts: &[&str]) {
        assert_eq!(self.code, code, "exit code; output:\n{}", self.output);
        for part in parts {
            assert!(
                self.output.contains(part),
                "the output lacks {part:?}:\n{}",
                self.output
            );
        }
    }
}
