//! A run: reading its arguments, checking each pin, and moving the differing ones.

use std::env;

use super::{Failure, Pins, Request, Scratch, Status};
use crate::git::Git;

/// The status `git grep` exits with where nothing matches, which only means no tracked
/// file spells a pin.
const GREP_NO_MATCH: i32 = 1;

impl<'a> Request<'a> {
    /// Reads `args`: `--bump` first, and then git pathspecs.
    pub(crate) fn new(args: &'a [String]) -> Self {
        let bump = args.first().is_some_and(|first| first == "--bump");
        Request {
            bump,
            pathspecs: if bump { &args[1..] } else { args },
        }
    }

    /// Runs the check, and returns how it ended.
    pub(crate) fn run(&self) -> Status {
        match self.check() {
            Ok(status) | Err(status) => status,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a status it has
    /// already reported.
    fn check(&self) -> Result<Status, Status> {
        self.refuse_options()?;
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
        Ok(status.max(differing.move_pins(&scratch)?))
    }

    fn refuse_options(&self) -> Result<(), Status> {
        match self.pathspecs.iter().find(|arg| arg.starts_with('-')) {
            Some(option) => {
                Failure(&format!(
                    "unknown argument '{option}'; this takes --bump, then git pathspecs"
                ))
                .report();
                Err(Status::Unknown)
            }
            None => Ok(()),
        }
    }

    /// The tracked files that spell a marker, named from the repository's root, where the
    /// rest of the run then works, so that a file reads the same from the working tree and
    /// from the default branch's tree.
    fn pinned_files(&self, git: &Git) -> Result<Vec<String>, Status> {
        let mut grep = vec!["grep", "-lzI", "--full-name", "-e", "git::", "--"];
        grep.extend(self.pathspecs.iter().map(String::as_str));
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
