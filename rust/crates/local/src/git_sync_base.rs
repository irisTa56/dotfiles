//! `git-sync-base`: brings a base branch up to date with origin, typically after a
//! pull request into it merges.

use std::io::{self, Write};

use crate::failure::Failure;
use crate::git::{git, read};

const USAGE: &str = "\
Usage: git-sync-base [branch]

Brings a base branch up to date with origin, typically after a pull request into it
merges: the branch named, or origin's default branch where none is.

Where no other worktree has the branch checked out, this checkout switches to it and
fast-forwards. Where another worktree has it, git refuses that switch, so the other
worktree is fast-forwarded in place and this checkout stays on its current branch.

  --help, -h  print this message
";

/// Brings the base branch `args` names, or origin's default one, up to date with
/// origin.
pub(crate) fn run(args: &[String]) -> Result<(), Failure> {
    match args {
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        [] => sync(None),
        [name] if !name.starts_with('-') => {
            sync(Some(name.as_str()).filter(|name| !name.is_empty()))
        }
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    }
}

/// Fetches origin and fast-forwards `named`, or origin's default branch, where it is
/// checked out.
///
/// In dotfiles' main checkout, the switch to main and the fast-forward of it fire the
/// hooks that reinstall dotfiles-local, which put a new file in this binary's place.
/// The run keeps the file it started from, so it ends as it would have otherwise.
fn sync(named: Option<&str>) -> Result<(), Failure> {
    // Always, so that a default branch origin changed since the clone is the one taken.
    git(&[
        "-c",
        "remote.origin.followRemoteHEAD=always",
        "fetch",
        "origin",
    ])?;
    let branch = base(named)?;
    let upstream = format!("origin/{branch}");
    let worktrees = read(&["worktree", "list", "--porcelain"])?;
    let holder = holder(&worktrees, &branch);
    let current = read(&["branch", "--show-current"])?;
    match holder {
        Some(holder) if current != branch => {
            git(&["-C", holder, "merge", "--ff-only", &upstream])?;
            let current = if current.is_empty() {
                "a detached HEAD"
            } else {
                &current
            };
            let _ = writeln!(
                io::stderr(),
                "Fast-forwarded {branch} in {holder}; this checkout stays on {current}."
            );
            Ok(())
        }
        _ => {
            git(&["switch", &branch])?;
            git(&["merge", "--ff-only", &upstream])
        }
    }
}

/// The branch `named`, or origin's default one, without the `origin/` a remote-tracking
/// name starts with.
fn base(named: Option<&str>) -> Result<String, Failure> {
    let branch = match named {
        Some(name) => name.to_owned(),
        None => read(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])?,
    };
    Ok(branch.strip_prefix("origin/").unwrap_or(&branch).to_owned())
}

/// The path of the first worktree that `list`, git's porcelain worktree list, shows with
/// `branch` checked out.
fn holder<'a>(list: &'a str, branch: &str) -> Option<&'a str> {
    let checked_out = format!("branch refs/heads/{branch}");
    let mut path = None;
    for line in list.lines() {
        if let Some(worktree) = line.strip_prefix("worktree ") {
            path = Some(worktree);
        } else if line == checked_out {
            return path;
        }
    }
    None
}
