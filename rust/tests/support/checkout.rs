//! The repository a run of the binary checks, and how the run ended.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::process::{Command, Stdio};

use tempfile::TempDir;

use super::git::{IDENTITY, git, isolate};
use super::origin::Origin;

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
            checkout.write(path, content);
        }
        checkout.git(&["add", "-A"]);
        checkout
    }

    /// Writes `contents` to `path` in the working tree, staging nothing.
    pub(crate) fn write(&self, path: &str, contents: impl AsRef<[u8]>) {
        let file = self.dir.path().join(path);
        fs::create_dir_all(file.parent().expect("a file has a parent"))
            .expect("the file's directory can be made");
        fs::write(&file, contents).expect("the file can be written");
    }

    /// The bytes of `path` in the working tree.
    pub(crate) fn read(&self, path: &str) -> Vec<u8> {
        fs::read(self.dir.path().join(path)).expect("the file can be read")
    }

    /// Runs git in the repository, as setup that must succeed, and returns its trimmed
    /// stdout.
    pub(crate) fn git(&self, args: &[&str]) -> String {
        git(self.dir.path(), args)
    }

    /// Commits what is staged and makes it `origin`'s `main`.
    pub(crate) fn publish(&self, origin: &Origin) {
        self.git(&[&IDENTITY[..], &["commit", "-qm", "base"]].concat());
        self.git(&["branch", "-M", "main"]);
        self.git(&["remote", "add", "origin", origin.location()]);
        self.git(&["push", "-q", "origin", "main"]);
    }

    /// Puts the repository on `origin`'s `main` with its own files staged over it, so
    /// that the checkout differs from origin.
    pub(crate) fn stage_over(&self, origin: &Origin) {
        self.git(&["remote", "add", "origin", origin.location()]);
        self.git(&["fetch", "-q", "origin", "main"]);
        self.git(&["reset", "-q", "origin/main"]);
        self.git(&["add", "-A"]);
    }

    /// Runs `pin-check` with `args` in the repository.
    pub(crate) fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[])
    }

    /// Runs `pin-check` with `args` in the repository, with each variable of `env` set
    /// over the isolated environment, or removed from it where its value is `None`.
    pub(crate) fn run_with(&self, args: &[&str], env: &[(&str, Option<&OsStr>)]) -> Run {
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
        for (name, value) in env {
            match value {
                Some(value) => command.env(name, value),
                None => command.env_remove(name),
            };
        }
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
