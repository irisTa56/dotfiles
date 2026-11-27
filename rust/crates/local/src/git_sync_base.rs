//! `git-sync-base`: brings a base branch up to date with origin, typically after a
//! pull request into it merges.

use std::io::{self, Write};
use std::str::FromStr;

use clap::Args;
use dotfiles_shared::Failure;

use crate::git::{command, git, read};
use crate::program::exit_code;

/// Bring a base branch up to date with origin, typically after a pull request into it
/// merges.
///
/// Brings up to date the branch named, or origin's default branch where none is. Where no
/// other worktree has the branch checked out, this checkout switches to it and
/// fast-forwards. Where another worktree has it, git refuses that switch, so the other
/// worktree is fast-forwarded in place and this checkout stays on its current branch.
///
/// Where the branch has commits origin lacks and origin has moved on as well, those
/// commits are rebased onto origin's branch instead, and their hashes before and after
/// are printed. Where the worktree holding the branch has uncommitted changes to tracked
/// files, or the rebase conflicts, the branch is left as it was and the run fails.
#[derive(Debug, Args)]
pub(crate) struct Request {
    /// The base branch [default: origin's default branch]
    branch: Option<Branch>,
}

/// A branch's name, which may not open with `-`, since git would take it for an
/// option.
#[derive(Debug, Clone)]
struct Branch(String);

impl FromStr for Branch {
    type Err = &'static str;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if name.starts_with('-') {
            return Err("a branch may not open with '-', which git would take for an option");
        }
        Ok(Branch(name.to_owned()))
    }
}

impl Request {
    /// Brings the base branch the request names, or origin's default one, up to date
    /// with origin.
    pub(crate) fn run(self) -> Result<(), Failure> {
        let named = self.branch.map(|branch| branch.0);
        sync(named.as_deref().filter(|name| !name.is_empty()))
    }
}

/// Fetches origin and brings `named`, or origin's default branch, up to it where it is
/// checked out.
///
/// In dotfiles' main checkout, the switch to main and the fast-forward or rebase of it
/// fire the hooks that reinstall dotfiles-local, which put a new file in this binary's place.
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
            let done = Base::new(Some(holder), &branch, &upstream).bring_up()?;
            let current = if current.is_empty() {
                "a detached HEAD"
            } else {
                &current
            };
            let _ = writeln!(
                io::stderr(),
                "{done} {branch} in {holder}; this checkout stays on {current}."
            );
            Ok(())
        }
        _ => {
            git(&["switch", &branch])?;
            Base::new(None, &branch, &upstream).bring_up().map(|_| ())
        }
    }
}

/// A base branch where it is checked out, and the remote-tracking branch it is brought
/// up to.
struct Base<'a> {
    /// The worktree holding the branch, or none for this checkout.
    dir: Option<&'a str>,
    branch: &'a str,
    upstream: &'a str,
}

impl<'a> Base<'a> {
    fn new(dir: Option<&'a str>, branch: &'a str, upstream: &'a str) -> Self {
        Base {
            dir,
            branch,
            upstream,
        }
    }

    /// Fast-forwards the branch to its upstream, or, where the branch has commits the
    /// upstream lacks and the upstream has moved on as well, rebases those commits
    /// onto it, returning which of the two it did.
    fn bring_up(&self) -> Result<&'static str, Failure> {
        if self.only_upstream_has()?.is_empty() || self.only_branch_has()?.is_empty() {
            git(&self.at(&["merge", "--ff-only", self.upstream]))?;
            return Ok("Fast-forwarded");
        }
        let before = self.only_branch_has()?;
        if !read(&self.at(&["status", "--porcelain", "--untracked-files=no"]))?.is_empty() {
            return Err(Failure::new(
                format_args!(
                    "{} has uncommitted changes to tracked files, so its commits {} lacks are not rebased onto it:\n{}",
                    self.branch,
                    self.upstream,
                    indent(&before)
                ),
                1,
            ));
        }
        // Not stashed around the rebase, since every worktree shares the stash.
        let rebase = command(&self.at(&["rebase", "--no-autostash", self.upstream])).status()?;
        if !rebase.success() {
            git(&self.at(&["rebase", "--abort"]))?;
            return Err(Failure::new(
                format_args!(
                    "{0} has commits {1} lacks that do not rebase onto it cleanly, so {0} is left as it was:\n{2}",
                    self.branch,
                    self.upstream,
                    indent(&before)
                ),
                exit_code(rebase),
            ));
        }
        let after = self.only_branch_has()?;
        let _ = writeln!(
            io::stderr(),
            "Rebased the commits {} had that {} lacked onto it.\nBefore:\n{}\nAfter:\n{}",
            self.branch,
            self.upstream,
            indent(&before),
            indent(&after)
        );
        Ok("Rebased")
    }

    /// The commits the branch has that its upstream lacks, one `<hash> <subject>` line
    /// each, newest first.
    fn only_branch_has(&self) -> Result<String, Failure> {
        let range = format!("{}..{}", self.upstream, self.branch);
        read(&self.at(&["log", "--format=%h %s", &range]))
    }

    /// The commits the upstream has that the branch lacks, as [`Self::only_branch_has`]
    /// lists them.
    fn only_upstream_has(&self) -> Result<String, Failure> {
        let range = format!("{}..{}", self.branch, self.upstream);
        read(&self.at(&["log", "--format=%h %s", &range]))
    }

    /// `args` for git, run in the worktree holding the branch.
    fn at<'b>(&'b self, args: &[&'b str]) -> Vec<&'b str> {
        let mut all = Vec::new();
        if let Some(dir) = self.dir {
            all.extend(["-C", dir]);
        }
        all.extend_from_slice(args);
        all
    }
}

/// `lines` with each line indented two spaces.
fn indent(lines: &str) -> String {
    lines
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
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
