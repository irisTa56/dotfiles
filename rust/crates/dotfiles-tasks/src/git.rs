//! Runs git and hands back what it printed, whatever its exit status.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The exit status given to a git that could not be started, or that a signal stopped:
/// the same "could not tell" a task exits with.
const NO_STATUS: i32 = 2;

/// What one git run printed, and how it exited.
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) status: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Run {
    pub(crate) fn success(&self) -> bool {
        self.status == 0
    }
}

/// A git whose runs leave out the variables that tie git to one repository.
///
/// A git hook runs with `GIT_DIR` and its kin exported, which would point every run here,
/// the ones in scratch repositories included, at the repository the hook runs for.
#[derive(Debug)]
pub(crate) struct Git {
    cleared: Vec<String>,
}

impl Git {
    pub(crate) fn new() -> Self {
        let mut git = Git {
            cleared: Vec::new(),
        };
        let names = git.run(&["rev-parse", "--local-env-vars"]).stdout;
        git.cleared = names.lines().map(str::to_owned).collect();
        git
    }

    /// Runs git with `args` in the current directory.
    pub(crate) fn run(&self, args: &[&str]) -> Run {
        self.call(args).run()
    }

    /// Starts a run that the returned [`Call`] can give a directory, variables and input.
    pub(crate) fn call<'a>(&'a self, args: &'a [&'a str]) -> Call<'a> {
        Call {
            git: self,
            args,
            dir: None,
            env: Vec::new(),
            input: None,
        }
    }
}

/// One git run being set up.
#[derive(Debug)]
pub(crate) struct Call<'a> {
    git: &'a Git,
    args: &'a [&'a str],
    dir: Option<&'a Path>,
    env: Vec<(&'a str, &'a str)>,
    input: Option<&'a str>,
}

impl<'a> Call<'a> {
    pub(crate) fn dir(mut self, dir: &'a Path) -> Self {
        self.dir = Some(dir);
        self
    }

    pub(crate) fn env(mut self, name: &'a str, value: &'a str) -> Self {
        self.env.push((name, value));
        self
    }

    pub(crate) fn input(mut self, input: &'a str) -> Self {
        self.input = Some(input);
        self
    }

    pub(crate) fn run(self) -> Run {
        let mut command = Command::new("git");
        command.args(self.args);
        for name in &self.git.cleared {
            command.env_remove(name);
        }
        command.envs(self.env.iter().copied());
        if let Some(dir) = self.dir {
            command.current_dir(dir);
        }
        command
            .stdin(if self.input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = match command.spawn() {
            Ok(child) => child,
            Err(error) => return unstarted(&error),
        };
        let output = match feed(child, self.input) {
            Ok(output) => output,
            Err(error) => return unstarted(&error),
        };
        Run {
            status: output.status.code().unwrap_or(NO_STATUS),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

fn feed(
    mut child: std::process::Child,
    input: Option<&str>,
) -> std::io::Result<std::process::Output> {
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(input.as_bytes())?;
    }
    child.wait_with_output()
}

fn unstarted(error: &std::io::Error) -> Run {
    Run {
        status: NO_STATUS,
        stdout: String::new(),
        stderr: format!("could not run git: {error}\n"),
    }
}
