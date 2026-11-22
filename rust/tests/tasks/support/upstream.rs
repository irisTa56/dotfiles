//! Upstream repositories a pin names, reached by `file://` URLs.

use tempfile::TempDir;

use super::git::git;
use super::import::{Change, commit_on_main};

/// A commit spelled as a pin can spell one, which no upstream in these tests has.
pub(crate) const ABSENT_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// An upstream repository on `main`, reached by a `file://` URL.
pub(crate) struct Upstream {
    dir: TempDir,
    commits: Vec<String>,
}

impl Upstream {
    /// A repository whose `lints` changes in each of its `commits` commits.
    pub(crate) fn changing_lints(commits: usize) -> Self {
        let contents: Vec<String> = (0..commits).map(|commit| commit.to_string()).collect();
        let files: Vec<[(&str, Option<&str>); 1]> = contents
            .iter()
            .map(|content| [("lints/a.toml", Some(content.as_str()))])
            .collect();
        let changes: Vec<Change<'_>> = files.iter().map(|file| &file[..]).collect();
        Upstream::new(&changes)
    }

    /// A repository with one commit per item of `changes`, in order, made as `git commit`
    /// would make them, by the setup identity at the current time, but in one run of git.
    pub(crate) fn new(changes: &[Change<'_>]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        git(dir.path(), &["init", "-q", "-b", "main"]);
        let commits = commit_on_main(dir.path(), "c", changes);
        Upstream { dir, commits }
    }

    pub(crate) fn url(&self) -> String {
        format!("file://{}", self.dir.path().display())
    }

    /// The `index`th commit, counting from the first.
    pub(crate) fn commit(&self, index: usize) -> &str {
        &self.commits[index]
    }

    /// The marker that pins `path` at the `index`th commit.
    pub(crate) fn marker(&self, path: &str, index: usize) -> String {
        marker(&self.url(), path, &self.commits[index])
    }
}

/// A marker as the tracked files spell one.
pub(crate) fn marker(url: &str, path: &str, commit: &str) -> String {
    format!("git::{url}//{path}?ref={commit}")
}
