//! Fetching upstream commits into the scratch repository, and checking a pin there.

use std::cell::RefCell;
use std::path::Path;

use dotfiles_shared::Failure;

use super::{Checked, Compared, FailLine, Pin, Scratch, Status};
use crate::git::{Git, Run};
use crate::stream::Stream;

/// The ref a fetch of both a pin's commits leaves the default branch's tip at, since a
/// fetch of several commits names each in `FETCH_HEAD` in no order git documents.
const TIP_REF: &str = "refs/upstream-pin-check/tip";

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
    /// An empty repository in a directory of its own, removed when the value is dropped.
    pub(super) fn new(git: Git) -> Result<Self, Failure> {
        let dir = tempfile::Builder::new()
            .prefix("upstream-pin-check-")
            .tempdir()
            .map_err(|error| Status::stopped_on(&error))?;
        let scratch = Scratch {
            git,
            dir,
            fetched: RefCell::default(),
        };
        let init = scratch.git(&["init", "-q"]);
        if !init.success() {
            return Err(FailLine(&format!(
                "could not create a scratch repository at {}",
                scratch.dir().display()
            ))
            .ending_git(&init));
        }
        Ok(scratch)
    }

    pub(super) fn dir(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) -> Run {
        self.git.call(args).dir(self.dir()).run()
    }

    /// Fetches `refspecs` of `url`; only the trees are compared, so no file's content is
    /// downloaded. The scratch repository is removed after the run, so git's upkeep of it
    /// after a fetch would be work for nothing.
    fn fetch(&self, url: &str, refspecs: &[&str]) -> Run {
        let mut args = vec![
            "-c",
            "maintenance.auto=false",
            "fetch",
            "-q",
            "--depth=1",
            "--filter=blob:none",
            url,
        ];
        args.extend(refspecs);
        self.git(&args)
    }

    /// Fetches `commit` of `url`, and returns it as a full sha.
    fn fetch_one(&self, url: &str, commit: &str) -> Result<String, Run> {
        let fetched = self.fetch(url, &[commit]);
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

    /// Fetches `commit` of `url` unless this scratch holds it already, and returns whether
    /// it holds it now.
    pub(super) fn holds(&self, url: &str, commit: &str) -> bool {
        if self
            .fetched
            .borrow()
            .contains(&(url.to_owned(), commit.to_owned()))
        {
            return true;
        }
        let held = self.fetch_one(url, commit).is_ok();
        if held {
            self.remember(url, commit);
        }
        held
    }

    fn remember(&self, url: &str, commit: &str) {
        self.fetched
            .borrow_mut()
            .insert((url.to_owned(), commit.to_owned()));
    }

    /// The trees at `path` in `commits`, each `None` where `path` is not in it.
    pub(super) fn trees_at<const N: usize>(
        &self,
        commits: [&str; N],
        path: &str,
    ) -> [Option<String>; N] {
        let revs = commits.map(|commit| format!("{commit}:{path}"));
        self.resolve(revs.each_ref().map(String::as_str))
    }

    /// The object each of `revs` names, `None` for one that names none. Where every one
    /// names one, a single run of git says so; otherwise each is resolved on its own, as
    /// it is where that run prints other than one name per rev, which a rev it read as a
    /// range would.
    fn resolve<const N: usize>(&self, revs: [&str; N]) -> [Option<String>; N] {
        if N > 1 {
            let mut args = vec!["rev-parse"];
            args.extend(revs);
            args.push("--");
            let resolved = self.git(&args);
            let stdout = resolved.stdout();
            // rev-parse echoes the `--` that ends the revs after their names.
            let names: Vec<&str> = stdout.lines().take_while(|line| *line != "--").collect();
            if resolved.success() && names.len() == N {
                return std::array::from_fn(|i| Some(names[i].to_owned()));
            }
        }
        revs.map(|rev| {
            let object = self.git(&["rev-parse", "-q", "--verify", rev]);
            object.success().then(|| object.stdout().trim().to_owned())
        })
    }

    /// What `pin` is compared by, read in one fetch of both its commits; `None` where
    /// that fetch fails, which [`Scratch::fetch_each`] then tells apart.
    fn fetch_both(&self, pin: &Pin) -> Option<Compared> {
        let tip_refspec = format!("+HEAD:{TIP_REF}");
        if !self.fetch(&pin.url, &[&tip_refspec, &pin.commit]).success() {
            return None;
        }
        self.remember(&pin.url, &pin.commit);
        let at_tip = format!("{TIP_REF}:{}", pin.path);
        let pinned = format!("{}:{}", pin.commit, pin.path);
        let [tip, at_tip, pinned] = self.resolve([TIP_REF, &at_tip, &pinned]);
        Some(Compared {
            tip: tip?,
            at_tip,
            pinned,
        })
    }

    /// What `pin` is compared by, fetching each of its commits on its own, so that a run
    /// that cannot read one says which, and why.
    fn fetch_each(&self, pin: &Pin) -> Result<Compared, Checked> {
        let tip = self.fetch_one(&pin.url, "HEAD").map_err(|run| {
            FailLine(&format!("could not read the default branch of {}", pin.url)).report_git(&run);
            Checked::Unknown
        })?;
        let pinned = self.fetch_one(&pin.url, &pin.commit).map_err(|run| {
            FailLine(&format!("could not read {} from {}", pin.commit, pin.url)).report_git(&run);
            Checked::Unknown
        })?;
        self.remember(&pin.url, &pin.commit);
        let [at_tip, pinned] = self.trees_at([&tip, &pinned], &pin.path);
        Ok(Compared {
            tip,
            at_tip,
            pinned,
        })
    }

    pub(super) fn check(&self, pin: &Pin) -> Checked {
        let place = format!("{}//{}", pin.url, pin.path);
        let compared = match self.fetch_both(pin) {
            Some(compared) => compared,
            None => match self.fetch_each(pin) {
                Ok(compared) => compared,
                Err(checked) => return checked,
            },
        };
        let Compared {
            tip,
            at_tip,
            pinned,
        } = compared;
        let Some(pinned_tree) = pinned else {
            FailLine(&format!(
                "{place} does not exist at the pinned {}",
                pin.commit
            ))
            .report();
            return Checked::Unknown;
        };
        let files = pin.files.join(", ");
        // A pin moved to a tip without the path would fail every run after, so it stays.
        let Some(tip_tree) = at_tip else {
            FailLine(&format!(
                "{place} is gone from the default branch ({tip}), pinned in {files}; a bump leaves this pin where it is"
            )).report();
            return Checked::Gone;
        };
        if pinned_tree == tip_tree {
            Stream::Stdout.line(format_args!(
                "[ok] {place} at the pinned {} matches the default branch ({tip})",
                pin.commit
            ));
            return Checked::Matches;
        }
        // Not which is the older: the pin may be ahead of the default branch as well as behind.
        FailLine(&format!(
            "{place} differs from the default branch, pinned in {files}"
        ))
        .report();
        FailLine(&format!("pinned: {}", pin.commit)).report();
        FailLine(&format!("tip:    {tip}")).report();
        Checked::Differs(tip)
    }
}
