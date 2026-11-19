//! The repository a run of the binary checks, and how the run ended.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::process::{Command, Stdio};

use tempfile::TempDir;

use super::git::{git, isolate};

/// The repository a run checks: a fresh git repository with the files a test stages.
pub(crate) struct Checkout {
    dir: TempDir,
}

/// How a run of the binary ended: its exit code, and stdout and stderr as one stream.
pub(crate) struct Run {
    pub(crate) code: i32,
    pub(crate) output: String,
}

impl Checkout {
    /// A repository holding `files`, given as path and content, staged.
    pub(crate) fn new(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let checkout = Checkout { dir };
        checkout.git(&["init", "-q"]);
        for (path, content) in files {
            let file = checkout.dir.path().join(path);
            fs::create_dir_all(file.parent().expect("a file has a parent"))
                .expect("the file's directory can be made");
            fs::write(&file, content).expect("the file can be written");
        }
        checkout.git(&["add", "-A"]);
        checkout
    }

    /// Runs git in the repository, as setup that must succeed.
    pub(crate) fn git(&self, args: &[&str]) {
        git(self.dir.path(), args);
    }

    /// Runs `pin-check` with `args` in the repository.
    pub(crate) fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[])
    }

    /// Runs `pin-check` with `args` in the repository, with `env` set over the
    /// isolated environment.
    pub(crate) fn run_with(&self, args: &[&str], env: &[(&str, &OsStr)]) -> Run {
        let (mut reader, writer) = io::pipe().expect("a pipe can be made");
        let mut command = Command::new(env!("CARGO_BIN_EXE_dotfiles-tasks"));
        command
            .arg("pin-check")
            .args(args)
            .current_dir(self.dir.path())
            .stdin(Stdio::null())
            .stdout(writer.try_clone().expect("the pipe can be shared"))
            .stderr(writer);
        isolate(&mut command, self.dir.path());
        command.envs(env.iter().copied());
        let mut child = command.spawn().expect("the binary runs");
        // The command holds the pipe's writing ends until it is dropped, and reading
        // waits for every one to close.
        drop(command);
        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .expect("the output is UTF-8");
        let status = child.wait().expect("the binary's run ends");
        Run {
            code: status
                .code()
                .expect("the binary exits rather than being killed"),
            output,
        }
    }
}

impl Run {
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
