//! Stand-ins for the programs a task runs, for the ends the real ones cannot be made to
//! reach on cue: a git that fails or is killed on one of its runs and is the real git on
//! the rest, or a lychee or curl that ends as a test needs.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::git::program;

/// What a stand-in runs to fail as a program says why and exits with a status of its
/// own: 5, which no task ends with of itself.
pub(crate) const FAILING: &str = "echo 'stand-in: failed' >&2\nexit 5\n";

/// What a stand-in runs to be stopped by a signal, as by an out-of-memory killer.
pub(crate) const KILLED: &str = "kill -9 $$\n";

/// A directory of stand-ins, each a script run by `sh`.
pub(crate) struct StandIns {
    dir: TempDir,
}

impl StandIns {
    pub(crate) fn new() -> Self {
        StandIns {
            dir: TempDir::new().expect("a temporary directory can be made"),
        }
    }

    /// Adds a `name` that runs `script`.
    pub(crate) fn program(self, name: &str, script: &str) -> Self {
        let file = self.dir.path().join(name);
        fs::write(&file, format!("#!/bin/sh\n{script}")).expect("the stand-in can be written");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
            .expect("the stand-in can be made executable");
        self
    }

    /// Adds a `name` that cannot be run, being no executable file.
    pub(crate) fn unrunnable(self, name: &str) -> Self {
        fs::write(self.dir.path().join(name), "").expect("the stand-in can be written");
        self
    }

    /// Adds a git that runs `script` where its arguments, joined by spaces, hold `args`,
    /// and runs the real git on the rest.
    pub(crate) fn git_on(self, args: &str, script: &str) -> Self {
        let real = program().display().to_string();
        self.program(
            "git",
            &format!("case \" $* \" in\n*'{args}'*)\n{script};;\nesac\nexec '{real}' \"$@\"\n"),
        )
    }

    /// The directory the stand-ins are in.
    pub(crate) fn dir(&self) -> &Path {
        self.dir.path()
    }

    /// The tests' PATH with the stand-ins ahead of the rest.
    pub(crate) fn path(&self) -> OsString {
        self.path_over(&std::env::var_os("PATH").expect("PATH is set"))
    }

    /// `path` with the stand-ins ahead of the rest.
    pub(crate) fn path_over(&self, path: &OsStr) -> OsString {
        let dirs =
            std::iter::once(PathBuf::from(self.dir.path())).chain(std::env::split_paths(path));
        std::env::join_paths(dirs).expect("no directory on PATH holds a colon")
    }
}
