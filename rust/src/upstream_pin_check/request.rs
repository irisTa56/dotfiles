//! A run: reading its arguments, checking each pin, and moving the differing ones.

use std::env;
use std::str::FromStr;

use super::{BUMP_BRANCH, Failure, Pathspec, Pins, Proposal, Request, Scratch, Status};
use crate::git::Git;

/// The status `git grep` exits with where nothing matches, which only means no tracked
/// file spells a pin.
const GREP_NO_MATCH: i32 = 1;

impl Request {
    /// Runs the check, and returns how it ended.
    pub(crate) fn run(&self) -> Status {
        match self.check() {
            Ok(status) | Err(status) => status,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a status it has
    /// already reported.
    fn check(&self) -> Result<Status, Status> {
        let git = Git::new();
        let files = self.pinned_files(&git)?;
        let files: Vec<&str> = files.iter().map(String::as_str).collect();
        let pins = Pins::read(&files).map_err(|error| Status::stopped_on(&error))?;
        if pins.0.is_empty() {
            Failure("found no upstream pin in the tracked files").report();
            return Err(Status::Unknown);
        }
        let scratch = Scratch::new(git)?;
        let (status, differing) = pins.check(&scratch);
        if !self.bump || differing.0.is_empty() {
            return Ok(status);
        }
        let branch = self.branch();
        let proposal = Proposal {
            branch: &branch,
            draft: self.draft,
        };
        Ok(status.max(differing.move_pins(&scratch, proposal)?))
    }

    /// The branch --bump proposes its move on under GitHub Actions.
    fn branch(&self) -> String {
        match &self.branch_suffix {
            Some(suffix) => format!("{BUMP_BRANCH}-{suffix}"),
            None => BUMP_BRANCH.to_owned(),
        }
    }

    /// The tracked files that spell a marker, named from the repository's root, where the
    /// rest of the run then works, so that a file reads the same from the working tree and
    /// from the default branch's tree.
    fn pinned_files(&self, git: &Git) -> Result<Vec<String>, Status> {
        let mut grep = vec!["grep", "-lzI", "--full-name", "-e", "git::", "--"];
        grep.extend(self.pathspecs.iter().map(|pathspec| pathspec.0.as_str()));
        let listed = git.run(&grep);
        if !listed.success() && listed.status() != Some(GREP_NO_MATCH) {
            return Err(
                Failure("could not list the tracked files to read pins from").report_git(&listed),
            );
        }
        let root = git.run(&["rev-parse", "--show-toplevel"]);
        if !root.success() {
            return Err(Failure("could not find the repository's root").report_git(&root));
        }
        env::set_current_dir(root.stdout().trim()).map_err(|error| Status::stopped_on(&error))?;
        Ok(listed
            .stdout()
            .split('\0')
            .filter(|f| !f.is_empty())
            .map(str::to_owned)
            .collect())
    }
}

impl FromStr for Pathspec {
    type Err = &'static str;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        if arg.starts_with('-') {
            return Err(
                "an option goes before the pathspecs; this takes --bump, --draft and --branch-suffix, then git pathspecs",
            );
        }
        Ok(Pathspec(arg.to_owned()))
    }
}
