//! The options a run is given.

use clap::Args;

use super::report::Report;

/// What the arguments of a run ask for.
#[derive(Debug, Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a switch of its own, as the arguments give it"
)]
pub(super) struct Options {
    /// Show every directory's table, and ask and upgrade nothing
    #[arg(short = 'n', long)]
    dry_run: bool,
    /// Also show for each tool the newest version overall, and the tools newer only past
    /// their request
    // The upgrade stays within each request, so a bump is only ever a report, and one
    // shown beside the question would read as an offer.
    #[arg(long, requires = "dry_run")]
    bump: bool,
    /// Also process linked git worktrees, such as agents' worktrees
    #[arg(long)]
    include_worktrees: bool,
    /// Also process configs under a temporary directory
    #[arg(long)]
    include_tmpdirs: bool,
}

impl Options {
    /// The `mise outdated` runs a run makes, and so the columns its tables have.
    pub(super) fn report(&self) -> Report {
        if self.bump {
            Report::Bump
        } else {
            Report::Pinned
        }
    }

    pub(super) fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub(super) fn include_worktrees(&self) -> bool {
        self.include_worktrees
    }

    pub(super) fn include_tmpdirs(&self) -> bool {
        self.include_tmpdirs
    }
}
