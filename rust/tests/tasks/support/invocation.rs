//! A run of the binary, a wrapper or a shell task in a checkout, put together one setting at
//! a time, and how the run ended.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use regex::Regex;

/// A run not yet started: the command, the checkout it runs in, what it is fed on stdin,
/// or the file it reads there instead, and which of its output streams, if any, is closed.
#[must_use = "an invocation runs nothing until `run` is called"]
pub(crate) struct Invocation {
    command: Command,
    root: PathBuf,
    input: Vec<u8>,
    stdin: Option<File>,
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

/// How a run ended: its exit code, and stdout and stderr each as it was written.
pub(crate) struct Apart {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Invocation {
    /// A run of `command`, which already runs in the checkout at `root`, fed nothing and
    /// with both streams read.
    pub(super) fn new(command: Command, root: PathBuf) -> Self {
        Invocation {
            command,
            root,
            input: Vec::new(),
            stdin: None,
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

    /// Gives the run the file at `path`, opened for reading, as its stdin, in place of a
    /// pipe it is fed through: a directory there makes every read fail.
    pub(crate) fn stdin_from(mut self, path: &Path) -> Self {
        self.stdin = Some(File::open(path).expect("the file can be opened"));
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
            mut command,
            input,
            stdin,
            ..
        } = self;
        let mut child = command
            .stdin(stdin.map_or_else(Stdio::piped, Stdio::from))
            .spawn()
            .expect("the command runs");
        // A command that exits without reading its input fails the write, which says
        // nothing about the run.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&input);
        }
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

    /// Runs the command to its end, feeding it its input and reading stdout and stderr
    /// apart. No stream is closed: a closed one has nothing to read.
    pub(crate) fn run_apart(self) -> Apart {
        let (status, stdout, stderr) = self.run_apart_status();
        Apart {
            code: status
                .code()
                .expect("the command exits rather than being killed"),
            stdout,
            stderr,
        }
    }

    /// Runs the command to its end, which is a signal's, and returns what it had written to
    /// stderr by then.
    pub(crate) fn run_killed(self) -> String {
        let (status, _stdout, stderr) = self.run_apart_status();
        assert_eq!(
            status.signal(),
            Some(9),
            "the command is killed; stderr:\n{stderr}"
        );
        stderr
    }

    fn run_apart_status(self) -> (ExitStatus, String, String) {
        assert!(self.closed.is_none(), "a run read apart closes no stream");
        let Invocation {
            mut command,
            input,
            stdin,
            ..
        } = self;
        let mut child = command
            .stdin(stdin.map_or_else(Stdio::piped, Stdio::from))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the command runs");
        // Fed apart from the reading, so that a run writing more than a pipe holds before
        // reading its input does not wait on the test. A command that exits without
        // reading its input fails the write, which says nothing about the run.
        let feeder = child.stdin.take().map(|mut stdin| {
            thread::spawn(move || {
                let _ = stdin.write_all(&input);
            })
        });
        let output = child.wait_with_output().expect("the command's run ends");
        if let Some(feeder) = feeder {
            feeder.join().expect("feeding the input does not panic");
        }
        (
            output.status,
            String::from_utf8(output.stdout).expect("stdout is UTF-8"),
            String::from_utf8(output.stderr).expect("stderr is UTF-8"),
        )
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

impl Apart {
    /// Asserts that the run exited with `code` and printed `stdout` and `stderr`, each
    /// whole.
    pub(crate) fn expect(&self, code: i32, stdout: &str, stderr: &str) {
        assert_eq!(
            (self.code, self.stdout.as_str(), self.stderr.as_str()),
            (code, stdout, stderr),
            "exit code, stdout and stderr"
        );
    }

    /// Asserts that the run exited with `code` and printed `stdout`, and on stderr what
    /// `stderr` matches whole, a pattern for output that names a path the run made up.
    pub(crate) fn expect_matching(&self, code: i32, stdout: &str, stderr: &str) {
        let pattern = Regex::new(&format!(r"\A(?:{stderr})\z")).expect("the pattern is valid");
        assert!(
            (self.code, self.stdout.as_str()) == (code, stdout) && pattern.is_match(&self.stderr),
            "expected exit code {code}, stdout {stdout:?} and stderr matching {pattern}, not {}, {:?} and {:?}",
            self.code,
            self.stdout,
            self.stderr
        );
    }
}
