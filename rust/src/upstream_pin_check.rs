//! `upstream:pin-check`: whether the upstream paths this repository pins have changed on
//! their default branches, and with `--bump`, moving the pins that have.
//!
//! What a pin is, and what each exit code means, is in tasks/README.md under
//! "upstream:pin-check". This module defines the values a run passes around; its child
//! modules provide what each does.

mod bump;
mod github;
mod moves;
mod pins;
mod report;
mod request;
mod scratch;
mod text;

use clap::Args;
use tempfile::TempDir;

use crate::git::Git;

/// How a run ends, ordered so that the worst of several outcomes is their maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Status {
    /// Nothing to act on.
    Ok = 0,
    /// A pinned path differs from its upstream's default branch.
    Differs = 1,
    /// It could not tell.
    Unknown = 2,
}

/// A run's arguments: `--bump`, `--draft` and `--branch-suffix`, and then git pathspecs.
#[derive(Debug, Args)]
pub(crate) struct Request {
    /// Move each differing pin to its upstream's tip, and under GitHub Actions push the
    /// move and open its pull request.
    #[arg(long)]
    bump: bool,
    /// Open the pull request of --bump as a draft, for a move a person has to finish
    /// before it merges; a pull request already open is left as it is, draft or not.
    #[arg(long, requires = "bump")]
    draft: bool,
    /// Push the move of --bump to `upstream-pin-bump-<suffix>` and open its pull request
    /// from there, so that pins proposed apart, in runs narrowed by pathspec, each have a
    /// pull request of their own.
    #[arg(long, requires = "bump", value_name = "SUFFIX")]
    branch_suffix: Option<String>,
    /// The files to read pins from, as git pathspecs [default: every tracked file].
    #[arg(trailing_var_arg = true)]
    pathspecs: Vec<Pathspec>,
}

/// The branch a bump under GitHub Actions pushes its move to and opens its pull request
/// from, with `-<suffix>` after it where `--branch-suffix` gives one. It is this task's
/// alone, so where it exists its pull request is open. A person may add commits to it, so
/// a later run adds its own over them and never forces; merging the pull request deletes
/// it where the repository deletes merged branches, and the next run pushes afresh.
const BUMP_BRANCH: &str = "upstream-pin-bump";

/// A git pathspec. One that opens with `-` is refused, since past the first pathspec
/// it is an option written out of place, such as a `--bump` written last.
#[derive(Debug, Clone)]
struct Pathspec(String);

/// What a run could not do, as the line that says so.
#[derive(Debug, Clone, Copy)]
struct Failure<'m>(&'m str);

/// A file's contents, in which markers are found and moved. They are bytes, since a file
/// need not be UTF-8, and a move leaves every byte outside its markers as it was.
#[derive(Debug, Clone, Copy)]
struct Text<'t>(&'t [u8]);

/// A marker as a text spells it: `git::<url>//<path>?ref=<commit>`.
#[derive(Debug, Clone, Copy)]
struct Marker<'t> {
    url: &'t str,
    path: &'t str,
    commit: &'t str,
}

/// One pin: an upstream URL, a path in it, and the commit pinned, with the tracked files
/// that spell it.
#[derive(Debug)]
struct Pin {
    url: String,
    path: String,
    commit: String,
    files: Vec<String>,
}

/// The pins the tracked files spell, each once, in the order first met.
#[derive(Debug)]
struct Pins(Vec<Pin>);

/// A pin that differs, and the default branch's tip it would move to.
#[derive(Debug)]
struct Move<'p> {
    pin: &'p Pin,
    tip: String,
}

/// The pins whose paths differ, which a bump moves.
#[derive(Debug)]
struct Differing<'p>(Vec<Move<'p>>);

/// A pin as one text spelled it, and the tip the move put there in its place.
#[derive(Debug)]
struct Moved {
    url: String,
    path: String,
    from: String,
    tip: String,
}

/// The commit message of a move under GitHub Actions: its subject, then a line per
/// distinct move.
#[derive(Debug)]
struct Message(Vec<String>);

/// What a bump moved, over however many files and markers.
#[derive(Debug, Default)]
struct Moves(Vec<Moved>);

/// What checking one pin found.
#[derive(Debug)]
enum Checked {
    Matches,
    /// The path is gone from the default branch, so there is nothing to move the pin to.
    Gone,
    /// The path differs from the one at this default branch tip.
    Differs(String),
    Unknown,
}

/// A scratch repository that upstream commits are fetched into, trees without blobs, and
/// removed with it.
#[derive(Debug)]
struct Scratch {
    git: Git,
    dir: TempDir,
}

/// GitHub's REST API as a run under GitHub Actions reaches it: where the API is, the
/// repository the run is for, and the token the job hands the step.
#[derive(Debug)]
struct GitHub {
    api: String,
    repository: String,
    token: String,
}

/// How a bump under GitHub Actions proposes its move: the branch it pushes to and opens
/// its pull request from, and whether that pull request is a draft.
#[derive(Debug, Clone, Copy)]
struct Proposal<'r> {
    branch: &'r str,
    draft: bool,
}

/// A pull request to open, from `head` into `base`, as a draft where `draft` is set.
#[derive(Debug)]
struct PullRequest<'a> {
    head: &'a str,
    base: &'a str,
    title: &'a str,
    body: &'a str,
    draft: bool,
}
