//! The review-loop skill's commands: `review-loop-record-paths` prints where the record
//! of the branch checked out lives, and `review-loop-record-selector-input` writes what
//! the close's selector reads.
//!
//! They run in the checkout the loop reviews, and the record is that branch's, so the
//! loop copies a path from their output rather than composing one.

pub(crate) mod paths;
pub(crate) mod selector_input;

use std::path::PathBuf;

use dotfiles_shared::Failure;

use crate::git;

/// The record of the branch checked out in the working directory: its verdict section
/// and its background, each a file under a directory of its own.
struct Record {
    /// The home directory, under whose `.claude/` the record lives.
    home: PathBuf,
    /// The directory holding the verdict sections of this repository's branches.
    verdicts: PathBuf,
    /// The directory holding the backgrounds of this repository's branches.
    backgrounds: PathBuf,
    /// The branch's name, which names both files.
    branch: String,
}

impl Record {
    /// The record of the branch checked out in the working directory, under `$HOME`.
    ///
    /// Both directories are keyed by the common git directory, which linked worktrees
    /// share, with each `/` of its absolute path as `-`.
    fn locate() -> Result<Self, Failure> {
        let home = std::env::var_os("HOME")
            .filter(|dir| !dir.is_empty())
            .ok_or_else(|| Failure::new(format_args!("HOME is not set"), 1))?;
        let common = git::read(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let key = common.replace('/', "-");
        let branch = git::read(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let home = PathBuf::from(home);
        let claude = home.join(".claude");
        Ok(Record {
            home,
            verdicts: claude.join("review-loop").join(&key),
            backgrounds: claude.join("review-loop-background").join(&key),
            branch,
        })
    }

    /// The verdict section's file.
    fn record(&self) -> PathBuf {
        self.verdicts.join(format!("{}.md", self.branch))
    }

    /// The background's file.
    fn background(&self) -> PathBuf {
        self.backgrounds.join(format!("{}.md", self.branch))
    }
}
