//! Running another program: every task starts the programs it runs through [`Program`],
//! which says the same of each one it cannot start, feed or wait on, and leaves what that
//! ends the task with to the task.

use std::ffi::OsStr;
use std::fmt;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};

/// A program to run, with its arguments, environment, directory and streams.
#[derive(Debug)]
pub(crate) struct Program {
    command: Command,
}

/// A program that has started, which can be given its input and waited on.
#[derive(Debug)]
pub(crate) struct Running {
    child: Child,
    program: String,
}

/// A program that could not be started, given its input or waited on, with why.
#[derive(Debug)]
pub(crate) struct Unstarted {
    program: String,
    error: io::Error,
}

impl Program {
    pub(crate) fn new(program: impl AsRef<OsStr>) -> Self {
        Program {
            command: Command::new(program),
        }
    }

    pub(crate) fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.command.arg(arg);
        self
    }

    pub(crate) fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.command.args(args);
        self
    }

    pub(crate) fn envs<'a>(mut self, vars: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        self.command.envs(vars);
        self
    }

    pub(crate) fn env_remove(mut self, name: impl AsRef<OsStr>) -> Self {
        self.command.env_remove(name);
        self
    }

    pub(crate) fn current_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.command.current_dir(dir);
        self
    }

    pub(crate) fn stdin(mut self, stdin: impl Into<Stdio>) -> Self {
        self.command.stdin(stdin);
        self
    }

    pub(crate) fn stdout(mut self, stdout: impl Into<Stdio>) -> Self {
        self.command.stdout(stdout);
        self
    }

    pub(crate) fn stderr(mut self, stderr: impl Into<Stdio>) -> Self {
        self.command.stderr(stderr);
        self
    }

    /// Runs the program to its end, its streams inherited unless set.
    pub(crate) fn status(mut self) -> Result<ExitStatus, Unstarted> {
        let result = self.command.status();
        result.map_err(|error| self.unstarted(error))
    }

    /// Runs the program to its end, capturing its stdout and stderr where they are not
    /// set, and giving it no stdin unless set.
    pub(crate) fn output(mut self) -> Result<Output, Unstarted> {
        let result = self.command.output();
        result.map_err(|error| self.unstarted(error))
    }

    /// Starts the program, its streams inherited unless set.
    pub(crate) fn spawn(mut self) -> Result<Running, Unstarted> {
        match self.command.spawn() {
            Ok(child) => Ok(Running {
                child,
                program: self.name(),
            }),
            Err(error) => Err(self.unstarted(error)),
        }
    }

    fn unstarted(&self, error: io::Error) -> Unstarted {
        Unstarted {
            program: self.name(),
            error,
        }
    }

    fn name(&self) -> String {
        self.command.get_program().to_string_lossy().into_owned()
    }
}

impl Running {
    /// Writes `input` to the program's stdin and closes it, so that the program reads to
    /// its end.
    ///
    /// # Panics
    ///
    /// Where the program's stdin was not set to a pipe, or was fed already, since the
    /// input would otherwise be dropped and the program read some other stdin.
    pub(crate) fn feed(&mut self, input: &[u8]) -> Result<(), Unstarted> {
        let mut stdin = self
            .child
            .stdin
            .take()
            .expect("a program is fed once, through a stdin set to a pipe");
        stdin
            .write_all(input)
            .map_err(|error| self.unstarted(error))
    }

    /// Waits for the program to end.
    pub(crate) fn wait(mut self) -> Result<ExitStatus, Unstarted> {
        let result = self.child.wait();
        result.map_err(|error| self.unstarted(error))
    }

    /// Waits for the program to end, with what it printed on the streams set to pipes.
    pub(crate) fn wait_with_output(self) -> Result<Output, Unstarted> {
        let Running { child, program } = self;
        child
            .wait_with_output()
            .map_err(|error| Unstarted { program, error })
    }

    fn unstarted(&self, error: io::Error) -> Unstarted {
        Unstarted {
            program: self.program.clone(),
            error,
        }
    }
}

impl Unstarted {
    /// The status a shell ends with on a command it cannot run: 127 where the program is
    /// not found, and 126 where it is found and cannot be run.
    pub(crate) fn shell_status(&self) -> u8 {
        if self.error.kind() == io::ErrorKind::NotFound {
            127
        } else {
            126
        }
    }
}

impl fmt::Display for Unstarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not run {}: {}", self.program, self.error)
    }
}
