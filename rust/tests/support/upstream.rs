//! Upstream repositories a pin names, reached by `file://` URLs.

use std::fs;

use tempfile::TempDir;

use super::git::{IDENTITY, git};

/// An upstream repository on `main`, reached by a `file://` URL.
pub(crate) struct Upstream {
    dir: TempDir,
    commits: Vec<String>,
}

/// One upstream commit: each path with its content, `None` deleting the path.
pub(crate) type Change<'c> = &'c [(&'c str, Option<&'c str>)];

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

    /// A repository with one commit per item of `changes`, in order.
    pub(crate) fn new(changes: &[Change<'_>]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        git(dir.path(), &["init", "-q", "-b", "main"]);
        let commits = changes
            .iter()
            .map(|change| {
                for (path, content) in *change {
                    let file = dir.path().join(path);
                    match content {
                        Some(content) => {
                            fs::create_dir_all(file.parent().expect("a file has a parent"))
                                .expect("the file's directory can be made");
                            fs::write(&file, content).expect("the file can be written");
                        }
                        None => fs::remove_file(&file).expect("the file can be removed"),
                    }
                }
                git(dir.path(), &["add", "-A"]);
                git(
                    dir.path(),
                    &[&IDENTITY[..], &["commit", "-q", "--allow-empty", "-m", "c"]].concat(),
                );
                git(dir.path(), &["rev-parse", "HEAD"])
            })
            .collect();
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
