//! `git-sweep-branches`: runs gh-poi, then deletes the local branches it leaves that
//! carry no commits of their own.

use std::io::{self, Write};
use std::process::{Command, Stdio};

use crate::failure::Failure;
use crate::git::{git, read};

const USAGE: &str = "\
Usage: git-sweep-branches

Runs gh-poi, then deletes the local branches it leaves that carry no commits of their
own. gh-poi deletes a branch only once it finds a pull request holding the branch's
commits, so a branch that never got one stays however long ago its tip reached the
default branch. Here such a branch goes too. A branch `gh poi lock` holds is left alone
by both, as is one checked out in a worktree.

  --help, -h  print this message
";

/// Fetches origin, runs gh-poi, and deletes the branches origin's default branch holds,
/// or, given `--help`, prints how it is called.
pub(crate) fn run(args: &[String]) -> Result<(), Failure> {
    match args {
        [] => sweep(),
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    }
}

fn sweep() -> Result<(), Failure> {
    refresh()?;
    gh_poi()?;
    delete_merged()
}

/// Fetches origin, taking the default branch origin has now and dropping the
/// remote-tracking branches it no longer has, and forgets the worktrees whose
/// directories are gone, which would otherwise hold their branches.
fn refresh() -> Result<(), Failure> {
    git(&[
        "-c",
        "remote.origin.followRemoteHEAD=always",
        "fetch",
        "--prune",
        "origin",
    ])?;
    git(&["worktree", "prune"])
}

/// Runs gh-poi, which deletes the branches whose merged pull request holds them.
fn gh_poi() -> Result<(), Failure> {
    let status = Command::new("gh-poi")
        .status()
        .map_err(|error| Failure::new(format_args!("cannot run gh-poi: {error}"), 1))?;
    if status.success() {
        Ok(())
    } else {
        Err(Failure::passed_on(status))
    }
}

/// Deletes each local branch origin's default branch holds whole, except that default
/// branch itself, one checked out in a worktree, which git cannot delete, and one
/// `gh poi lock` holds.
fn delete_merged() -> Result<(), Failure> {
    let base = read(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])?;
    let default = base.strip_prefix("origin/").unwrap_or(&base);
    let worktrees = read(&["worktree", "list", "--porcelain"])?;
    let checked_out: Vec<&str> = worktrees
        .lines()
        .filter_map(|line| line.strip_prefix("branch refs/heads/"))
        .collect();
    let merged = read(&[
        "for-each-ref",
        "--merged",
        &base,
        "--format=%(refname:short)",
        "refs/heads",
    ])?;
    for branch in merged.lines() {
        if branch == default || checked_out.contains(&branch) || locked(branch) {
            continue;
        }
        // -D, since -d checks against the branch's upstream or HEAD rather than the
        // base, and --merged has just shown the base holds every commit on it.
        git(&["branch", "-D", branch])?;
    }
    Ok(())
}

/// Whether `gh poi lock` holds `branch`: its lock reads as true where git reads it as a
/// boolean, and a lock git cannot read as one holds nothing.
fn locked(branch: &str) -> bool {
    Command::new("git")
        .args([
            "config",
            "--type=bool",
            &format!("branch.{branch}.gh-poi-locked"),
        ])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| output.stdout == b"true\n")
}
