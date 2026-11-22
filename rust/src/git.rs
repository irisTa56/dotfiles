//! Runs git and hands back what it printed, whatever its exit status.

use std::borrow::Cow;
use std::path::Path;
use std::process::{ExitCode, Output, Stdio};

use crate::program::{Program, Unstarted};
use crate::stream::Stream;

/// What one git run printed, and how it exited.
#[derive(Debug)]
pub(crate) struct Run {
    /// None where git could not be started, or a signal stopped it.
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
}

impl Run {
    pub(crate) fn status(&self) -> Option<i32> {
        self.status
    }

    pub(crate) fn success(&self) -> bool {
        self.status == Some(0)
    }

    /// What git printed, as text: a name or an id, say.
    pub(crate) fn stdout(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// What git printed, byte for byte: a file's contents, which need not be UTF-8.
    pub(crate) fn stdout_bytes(&self) -> &[u8] {
        &self.stdout
    }

    pub(crate) fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Reports what a failed run said, and returns the code to end the task with: git's
    /// own status, such as 128 outside a repository, or `otherwise` where it has none,
    /// git having never run or been stopped by a signal.
    pub(crate) fn failure(&self, otherwise: u8) -> ExitCode {
        Stream::Stderr.bytes(self.stderr.as_bytes());
        ExitCode::from(
            self.status
                .and_then(|status| u8::try_from(status).ok())
                .unwrap_or(otherwise),
        )
    }

    /// The run of a git that could not be started, or not fed its input.
    fn unstarted(error: &Unstarted) -> Self {
        Run {
            status: None,
            stdout: Vec::new(),
            stderr: format!("{error}\n"),
        }
    }
}

/// A git, whose runs may leave out the variables that tie git to one repository.
///
/// A git hook runs with `GIT_DIR` and its kin exported, which would point every run,
/// the ones in scratch repositories included, at the repository the hook runs for.
#[derive(Debug)]
pub(crate) struct Git {
    cleared: Vec<String>,
}

impl Git {
    /// A git whose runs leave out those variables, for runs in repositories of its own.
    pub(crate) fn new() -> Self {
        let mut git = Git {
            cleared: Vec::new(),
        };
        let names = git.run(&["rev-parse", "--local-env-vars"]);
        git.cleared = names.stdout().lines().map(str::to_owned).collect();
        git
    }

    /// A git whose runs keep those variables, for a run about the repository the
    /// environment names: the index a pre-commit hook names is the one a commit is made
    /// from.
    pub(crate) fn inheriting() -> Self {
        Git {
            cleared: Vec::new(),
        }
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
    input: Option<&'a [u8]>,
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

    pub(crate) fn input(mut self, input: &'a [u8]) -> Self {
        self.input = Some(input);
        self
    }

    pub(crate) fn run(self) -> Run {
        match self.output() {
            Ok(output) => Run {
                status: output.status.code(),
                stdout: output.stdout,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
            Err(error) => Run::unstarted(&error),
        }
    }

    fn output(self) -> Result<Output, Unstarted> {
        let mut program = Program::new("git").args(self.args);
        for name in &self.git.cleared {
            program = program.env_remove(name);
        }
        program = program.envs(self.env.iter().copied());
        if let Some(dir) = self.dir {
            program = program.current_dir(dir);
        }
        let mut running = program
            .stdin(if self.input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(input) = self.input {
            running.feed(input)?;
        }
        running.wait_with_output()
    }
}
