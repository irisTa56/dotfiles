//! `dotfiles-tasks upstream-pin-check`, and the wrapper `tasks/upstream/pin-check.sh`.

mod bump;
mod check;
mod wrapper;

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::support::upstream::Upstream;

/// The trace2 events every git of a run writes, its children included, where the run is
/// given `GIT_TRACE2_EVENT` set to [`Trace::path`].
struct Trace {
    dir: TempDir,
}

impl Trace {
    fn new() -> Self {
        Trace {
            dir: TempDir::new().expect("a temporary directory can be made"),
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("events")
    }

    /// How many times `up` was asked for its refs or objects: each fetch from it starts
    /// one `git-upload-pack` there.
    fn requests_of(&self, up: &Upstream) -> usize {
        let url = up.url();
        let dir = Path::new(
            url.strip_prefix("file://")
                .expect("an upstream's URL is file://"),
        );
        let served = format!(r#""argv":["git-upload-pack","{}"]"#, dir.display());
        fs::read_to_string(self.path())
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains(r#""event":"start""#) && line.contains(&served))
            .count()
    }
}
