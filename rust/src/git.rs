//! Runs git and hands back what it printed, whatever its exit status.

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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

    /// The run of a git that could not be started, or not fed its input.
    fn unstarted(error: &std::io::Error) -> Self {
        Run {
            status: None,
            stdout: Vec::new(),
            stderr: format!("could not run git: {error}\n"),
        }
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
        let names = git.run(&["rev-parse", "--local-env-vars"]);
        git.cleared = names.stdout().lines().map(str::to_owned).collect();
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
            Err(error) => return Run::unstarted(&error),
        };
        let output = match feed(child, self.input) {
            Ok(output) => output,
            Err(error) => return Run::unstarted(&error),
        };
        Run {
            status: output.status.code(),
            stdout: output.stdout,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

fn feed(
    mut child: std::process::Child,
    input: Option<&[u8]>,
) -> std::io::Result<std::process::Output> {
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(input)?;
    }
    child.wait_with_output()
}
