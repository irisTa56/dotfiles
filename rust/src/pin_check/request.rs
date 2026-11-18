//! A run: reading its arguments, checking each pin, and moving the differing ones.

use std::{env, io};

use super::{Checked, Differing, Failure, Move, Pins, Request, Scratch, Status};
use crate::git::Git;

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
            Ok(status) => status,
            // An error from the filesystem is no answer about the pins, so it exits as one.
            Err(error) => {
                Failure(&format!("stopped on {error}")).report();
                Status::Unknown
            }
        }
    }

    fn check(&self) -> io::Result<Status> {
        if let Some(option) = self.pathspecs.iter().find(|arg| arg.starts_with('-')) {
            Failure(&format!(
                "unknown argument '{option}'; this takes --bump, then git pathspecs"
            ))
            .report();
            return Ok(Status::Unknown);
        }
        let git = Git::new();
        // Names from the repository's root, which is where the rest of the run works, so
        // that a file reads the same from the working tree and from the default branch's
        // tree.
        let mut grep = vec!["grep", "-lzI", "--full-name", "-e", "git::", "--"];
        grep.extend(self.pathspecs.iter().map(String::as_str));
        let listed = git.run(&grep);
        if listed.status() > 1 {
            return Ok(
                Failure("could not list the tracked files to read pins from").report_git(&listed),
            );
        }
        let root = git.run(&["rev-parse", "--show-toplevel"]);
        if !root.success() {
            return Ok(Failure("could not find the repository's root").report_git(&root));
        }
        env::set_current_dir(root.stdout().trim())?;
        let names = listed.stdout();
        let files: Vec<&str> = names.split('\0').filter(|f| !f.is_empty()).collect();
        let pins = Pins::read(&files)?;
        if pins.0.is_empty() {
            Failure("found no upstream pin in the tracked files").report();
            return Ok(Status::Unknown);
        }

        let scratch = Scratch::new(git)?;
        let init = scratch.git(&["init", "-q"]);
        if !init.success() {
            return Ok(Failure(&format!(
                "could not create a scratch repository at {}",
                scratch.dir().display()
            ))
            .report_git(&init));
        }
        let mut status = Status::Ok;
        let mut differing = Differing(Vec::new());
        for pin in &pins.0 {
            let checked = scratch.check(pin);
            status = status.max(checked.status());
            if let Checked::Differs(tip) = checked {
                differing.0.push(Move { pin, tip });
            }
        }
        if self.bump && !differing.0.is_empty() {
            let moved = if env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true") {
                scratch.move_on_default_branch(&differing)
            } else {
                differing.move_in_working_tree()?
            };
            status = status.max(moved);
        }
        Ok(status)
    }
}
