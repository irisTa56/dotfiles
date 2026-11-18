//! Fetching upstream commits into the scratch repository, and checking a pin there.

use std::io;
use std::path::Path;

use super::{Checked, Failure, Pin, Scratch, Status};
use crate::git::{Git, Run};

impl Checked {
    pub(super) fn status(&self) -> Status {
        match self {
            Checked::Matches => Status::Ok,
            Checked::Gone | Checked::Differs(_) => Status::Differs,
            Checked::Unknown => Status::Unknown,
        }
    }
}

impl Scratch {
    pub(super) fn new(git: Git) -> io::Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("upstream-pin-check-")
            .tempdir()?;
        Ok(Scratch { git, dir })
    }

    pub(super) fn dir(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn git(&self, args: &[&str]) -> Run {
        self.git.call(args).dir(self.dir()).run()
    }

    /// Fetches `commit` of `url`, and returns it as a full sha; only the trees are
    /// compared, so no file's content is downloaded.
    pub(super) fn fetch(&self, url: &str, commit: &str) -> Result<String, Run> {
        let fetched = self.git(&[
            "fetch",
            "-q",
            "--depth=1",
            "--filter=blob:none",
            url,
            commit,
        ]);
        if !fetched.success() {
            return Err(fetched);
        }
        let head = self.git(&["rev-parse", "FETCH_HEAD"]);
        if head.success() {
            Ok(head.stdout().trim().to_owned())
        } else {
            Err(head)
        }
    }

    pub(super) fn tree_at(&self, commit: &str, path: &str) -> Option<String> {
        let tree = self.git(&["rev-parse", "-q", "--verify", &format!("{commit}:{path}")]);
        tree.success().then(|| tree.stdout().trim().to_owned())
    }

    pub(super) fn check(&self, pin: &Pin) -> Checked {
        let place = format!("{}//{}", pin.url, pin.path);
        let tip = match self.fetch(&pin.url, "HEAD") {
            Ok(tip) => tip,
            Err(run) => {
                Failure(&format!("could not read the default branch of {}", pin.url))
                    .report_git(&run);
                return Checked::Unknown;
            }
        };
        let tip_tree = self.tree_at(&tip, &pin.path);
        let pinned = match self.fetch(&pin.url, &pin.commit) {
            Ok(pinned) => pinned,
            Err(run) => {
                Failure(&format!("could not read {} from {}", pin.commit, pin.url))
                    .report_git(&run);
                return Checked::Unknown;
            }
        };
        let Some(pinned_tree) = self.tree_at(&pinned, &pin.path) else {
            Failure(&format!(
                "{place} does not exist at the pinned {}",
                pin.commit
            ))
            .report();
            return Checked::Unknown;
        };
        let files = pin.files.join(", ");
        // A pin moved to a tip without the path would fail every run after, so it stays.
        let Some(tip_tree) = tip_tree else {
            Failure(&format!(
                "{place} is gone from the default branch ({tip}), pinned in {files}; a bump leaves this pin where it is"
            )).report();
            return Checked::Gone;
        };
        if pinned_tree == tip_tree {
            println!(
                "[ok] {place} at the pinned {} matches the default branch ({tip})",
                pin.commit
            );
            return Checked::Matches;
        }
        // Not which is the older: the pin may be ahead of the default branch as well as behind.
        Failure(&format!(
            "{place} differs from the default branch, pinned in {files}"
        ))
        .report();
        Failure(&format!("pinned: {}", pin.commit)).report();
        Failure(&format!("tip:    {tip}")).report();
        Checked::Differs(tip)
    }
}
