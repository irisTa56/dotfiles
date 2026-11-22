//! Runs git and hands back what it printed, whatever its exit status.

use std::borrow::Cow;
use std::path::Path;
use std::process::{ExitStatus, Output, Stdio};

use dotfiles_shared::Failure;

use crate::program::{self, Program, Unstarted};

/// What one git run printed, and how it exited.
#[derive(Debug)]
pub(crate) struct Run {
    /// None where git could not be started.
    status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: String,
}

impl Run {
    /// The status git exited with, None where it could not be started or a signal
    /// stopped it.
    pub(crate) fn status(&self) -> Option<i32> {
        self.status.and_then(|status| status.code())
    }

    pub(crate) fn success(&self) -> bool {
        self.status() == Some(0)
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

    /// What ends the task on a failed run, saying what git said: it ends with git's own
    /// status, such as 128 outside a repository, or `otherwise` where it has none, git
    /// having never run or been stopped by a signal.
    pub(crate) fn failure(&self, otherwise: u8) -> Failure {
        let code = self
            .status
            .map_or(otherwise, |status| program::exit_code(status, otherwise));
        Failure::verbatim(self.stderr.clone().into_bytes(), code)
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
                status: Some(output.status),
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

#[cfg(test)]
mod tests {
    use super::Git;

    /// A git that ends without reading its input leaves the write of that input failing,
    /// which the run holds as a git that could not run. The input is larger than any pipe
    /// holds, so the write cannot finish before git ends.
    #[test]
    fn holds_a_git_that_does_not_take_its_input_as_one_that_could_not_run() {
        let input = vec![b'x'; 8 << 20];
        let run = Git::inheriting().call(&["--version"]).input(&input).run();
        assert_eq!(
            (run.status(), run.stderr()),
            (None, "could not run git: Broken pipe (os error 32)\n")
        );
    }
}
