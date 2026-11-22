//! Upstream repositories a pin names, reached by `file://` URLs.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

use super::git::{IDENTITY_LINE, feed, git};

/// A commit spelled as a pin can spell one, which no upstream in these tests has.
pub(crate) const ABSENT_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// An upstream repository on `main`, reached by a `file://` URL.
pub(crate) struct Upstream {
    dir: TempDir,
    commits: Vec<String>,
}

/// One upstream commit: each path with its content, `None` deleting the path.
type Change<'c> = &'c [(&'c str, Option<&'c str>)];

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
        let marks = dir.path().join(".git/test-marks");
        feed(
            dir.path(),
            &[
                "fast-import",
                "--quiet",
                &format!("--export-marks={}", marks.display()),
            ],
            &import_stream(changes),
        );
        let marks = fs::read_to_string(&marks).expect("git wrote the marks");
        let mut commits: Vec<(usize, String)> = marks
            .lines()
            .map(|line| {
                let (mark, commit) = line.split_once(' ').expect("a mark names a commit");
                let mark = mark
                    .trim_start_matches(':')
                    .parse()
                    .expect("a mark is a number");
                (mark, commit.to_owned())
            })
            .collect();
        commits.sort_unstable();
        let commits = commits.into_iter().map(|(_, commit)| commit).collect();
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

/// The `git fast-import` stream that makes one commit on `main` per item of `changes`,
/// marked with its number counting from 1, each with the message `c`.
fn import_stream(changes: &[Change<'_>]) -> Vec<u8> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past the epoch")
        .as_secs();
    let mut stream = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        let mark = index + 1;
        stream.extend_from_slice(
            format!("commit refs/heads/main\nmark :{mark}\ncommitter {IDENTITY_LINE} {now} +0000\ndata 2\nc\n").as_bytes(),
        );
        for (path, content) in *change {
            match content {
                Some(content) => stream.extend_from_slice(
                    format!(
                        "M 100644 inline {path}\ndata {}\n{content}\n",
                        content.len()
                    )
                    .as_bytes(),
                ),
                None => stream.extend_from_slice(format!("D {path}\n").as_bytes()),
            }
        }
    }
    stream
}

/// A marker as the tracked files spell one.
pub(crate) fn marker(url: &str, path: &str, commit: &str) -> String {
    format!("git::{url}//{path}?ref={commit}")
}
