//! A stand-in for mise, so that the wrapper runs the binary this package builds rather
//! than installing a release.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

use tempfile::TempDir;

/// A directory holding a `mise` and a `dotfiles-tasks`, the latter the built binary.
pub(crate) struct Mise {
    dir: TempDir,
}

impl Mise {
    /// A mise whose `install` exits with `status`, and whose `where <tool>` prints this
    /// directory, which holds the built binary as `dotfiles-tasks`.
    pub(crate) fn installing_with(status: i32) -> Self {
        Self::stand_in(status, "exec dirname \"$0\"")
    }

    /// A mise whose `install` succeeds and whose `where` exits 1 printing nothing, as it
    /// does for a tool it has no installation of.
    pub(crate) fn installing_but_not_locating() -> Self {
        Self::stand_in(0, "exit 1")
    }

    /// A mise whose `install <tool>` exits with `status`, and whose `where <tool>` runs
    /// `locate` for the tool last installed and exits 65 for any other, so that a wrapper
    /// running another release than it installed fails.
    fn stand_in(status: i32, locate: &str) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let mise = dir.path().join("mise");
        let script = format!(
            "#!/bin/sh\n\
             for arg; do tool=$arg; done\n\
             installed=\"$(dirname \"$0\")/installed\"\n\
             [ \"$1\" = install ] && {{ printf %s \"$tool\" >\"$installed\"; exit {status}; }}\n\
             [ \"$1\" = where ] && {{ [ \"$tool\" = \"$(cat \"$installed\")\" ] || exit 65; {locate}; }}\n\
             exit 64\n"
        );
        fs::write(&mise, script).expect("the stand-in can be written");
        fs::set_permissions(&mise, fs::Permissions::from_mode(0o755))
            .expect("the stand-in can be made executable");
        symlink(
            env!("CARGO_BIN_EXE_dotfiles-tasks"),
            dir.path().join("dotfiles-tasks"),
        )
        .expect("the binary can be linked");
        Mise { dir }
    }

    /// The tests' own PATH, with this directory ahead of it.
    pub(super) fn path(&self) -> OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let dirs = std::iter::once(PathBuf::from(self.dir.path()))
            .chain(std::env::split_paths(&inherited));
        std::env::join_paths(dirs).expect("no directory on PATH holds a colon")
    }
}
