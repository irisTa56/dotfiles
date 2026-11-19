//! The bare repository a checkout pushes to, and reads of what a run pushed there.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tempfile::TempDir;

use super::checkout::Checkout;
use super::git::{IDENTITY, git, query};

/// A bare repository that a checkout names as its `origin`.
pub(crate) struct Origin {
    dir: TempDir,
}

impl Origin {
    /// An empty bare repository on `main`.
    pub(crate) fn new() -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        git(dir.path(), &["init", "-q", "--bare", "-b", "main"]);
        Origin { dir }
    }

    /// A bare repository whose `main` holds `files`, given as path and content, published
    /// from a checkout of its own.
    pub(crate) fn with_main(files: &[(&str, &str)]) -> Self {
        let origin = Origin::new();
        Checkout::new(files).publish(&origin);
        origin
    }

    /// Adds `count` empty commits on top of `main`, as other changes landing there.
    pub(crate) fn advance_main(&self, count: usize) {
        let work = TempDir::new().expect("a temporary directory can be made");
        git(work.path(), &["clone", "-q", self.location(), "."]);
        for _ in 0..count {
            git(
                work.path(),
                &[
                    &IDENTITY[..],
                    &["commit", "-q", "--allow-empty", "-m", "later"],
                ]
                .concat(),
            );
        }
        git(work.path(), &["push", "-q", "origin", "main"]);
    }

    /// Where a remote names the repository.
    pub(super) fn location(&self) -> &str {
        self.dir
            .path()
            .to_str()
            .expect("a temporary directory's path is UTF-8")
    }

    /// Makes the repository refuse every push to `branch`, as a token without
    /// contents: write gets refused.
    pub(crate) fn refuse_pushes_to(&self, branch: &str) {
        let hook = self.dir.path().join("hooks/pre-receive");
        fs::write(
            &hook,
            format!(
                "#!/bin/sh\nwhile read -r _ _ ref; do [ \"$ref\" = refs/heads/{branch} ] && exit 1; done\nexit 0\n"
            ),
        )
        .expect("the hook can be written");
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
            .expect("the hook can be made executable");
    }

    /// The bytes of `object`, as `git show` names one, or `None` where there is none.
    pub(crate) fn show(&self, object: &str) -> Option<Vec<u8>> {
        query(self.path(), &["show", object])
    }

    /// The mode `path` has at `rev`, as `git ls-tree` prints it, or `None` where it
    /// has no such path.
    pub(crate) fn mode(&self, rev: &str, path: &str) -> Option<String> {
        let listed = query(self.path(), &["ls-tree", rev, "--", path])?;
        let listed = String::from_utf8(listed).expect("git's listing is UTF-8");
        listed.split_whitespace().next().map(str::to_owned)
    }

    /// The commit `rev` names, or `None` where it names none.
    pub(crate) fn commit(&self, rev: &str) -> Option<String> {
        self.log(rev, "%H")
    }

    /// The last commit at `rev`, formatted as `format` says to `git log`, or `None`
    /// where `rev` names no commit.
    pub(crate) fn log(&self, rev: &str, format: &str) -> Option<String> {
        let out = query(
            self.path(),
            &["log", "-1", &format!("--format={format}"), rev, "--"],
        )?;
        Some(
            String::from_utf8(out)
                .expect("git's log is UTF-8")
                .trim()
                .to_owned(),
        )
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }
}
