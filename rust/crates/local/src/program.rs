//! Running another program: every command starts one through [`Program`], which reports
//! a program that cannot start, and this module turns how one ended into a failure.

use std::ffi::OsStr;
use std::fmt;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};

use dotfiles_shared::Failure;

/// A program to run, with its arguments, environment, directory and streams.
#[derive(Debug)]
pub(crate) struct Program {
    command: Command,
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

    pub(crate) fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.command.env(name, value);
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

    /// The command as set up, to show it.
    pub(crate) fn as_command(&self) -> &Command {
        &self.command
    }

    /// Runs the program to its end, its streams inherited unless set.
    pub(crate) fn status(mut self) -> Result<ExitStatus, Failure> {
        let result = self.command.status();
        result.map_err(|error| self.unrunnable(&error))
    }

    /// Runs the program to its end, capturing its stdout and stderr where they are not
    /// set.
    pub(crate) fn output(mut self) -> Result<Output, Failure> {
        let result = self.command.output();
        result.map_err(|error| self.unrunnable(&error))
    }

    /// Starts the program, its streams inherited unless set.
    pub(crate) fn spawn(mut self) -> Result<Child, Failure> {
        let result = self.command.spawn();
        result.map_err(|error| self.unrunnable(&error))
    }

    /// Runs the program, failing as it fails, where it says why itself.
    pub(crate) fn run(self) -> Result<(), Failure> {
        let status = self.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(passed_on(status))
        }
    }

    /// What the program prints on stdout, its stderr passed on, failing as it fails,
    /// where it says why itself.
    pub(crate) fn read(self) -> Result<Vec<u8>, Failure> {
        let output = self.stderr(Stdio::inherit()).output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(passed_on(output.status))
        }
    }

    fn unrunnable(&self, error: &std::io::Error) -> Failure {
        Failure::new(
            format_args!(
                "cannot run {}: {error}",
                self.command.get_program().to_string_lossy()
            ),
            1,
        )
    }
}

/// What ends a run where a program it ran ended with `status`, having said why on
/// stderr itself.
pub(crate) fn passed_on(status: ExitStatus) -> Failure {
    Failure::quiet(exit_code(status))
}

/// What ends a run where a program it ran ended with `status`, saying `what` failed and
/// the status it ended with.
pub(crate) fn failed(status: ExitStatus, what: fmt::Arguments<'_>) -> Failure {
    let code = exit_code(status);
    Failure::new(format_args!("{what} (exit {code})"), code)
}

/// The status a program ended with, a signal giving the one a shell gives.
pub(crate) fn exit_code(status: ExitStatus) -> u8 {
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1)
}
