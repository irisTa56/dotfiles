//! Commits made on a repository's `main` in one run of `git fast-import`, rather than in
//! an add and a commit per commit.

use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

use super::git::{IDENTITY_LINE, feed};

/// One commit: each path with its content, `None` deleting the path.
pub(crate) type Change<'c> = &'c [(&'c str, Option<&'c str>)];

/// Makes one commit on `main` in the repository at `dir` per item of `changes`, in order,
/// each with `message`, as `git commit` would make them by the setup identity at the
/// current time, and returns the commits.
pub(super) fn commit_on_main(dir: &Path, message: &str, changes: &[Change<'_>]) -> Vec<String> {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let marks = aux.path().join("marks");
    feed(
        dir,
        &[
            "fast-import",
            "--quiet",
            &format!("--export-marks={}", marks.display()),
        ],
        &stream(message, changes),
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
    commits.into_iter().map(|(_, commit)| commit).collect()
}

/// The stream that makes one commit on `main` per item of `changes`, each with `message`,
/// marked with its number counting from 1.
fn stream(message: &str, changes: &[Change<'_>]) -> Vec<u8> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past the epoch")
        .as_secs();
    let mut stream = String::new();
    for (index, change) in changes.iter().enumerate() {
        let mark = index + 1;
        // Writing to a String cannot fail.
        let _ = write!(
            stream,
            "commit refs/heads/main\nmark :{mark}\ncommitter {IDENTITY_LINE} {now} +0000\ndata {}\n{message}\n",
            message.len() + 1
        );
        for (path, content) in *change {
            let _ = match content {
                Some(content) => write!(
                    stream,
                    "M 100644 inline {path}\ndata {}\n{content}\n",
                    content.len()
                ),
                None => writeln!(stream, "D {path}"),
            };
        }
    }
    stream.into_bytes()
}
