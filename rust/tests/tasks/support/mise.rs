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
    /// A mise whose `install` exits with `status`, and whose `x <tool> -- <command>`
    /// runs the command alone, which finds the built binary as `dotfiles-tasks`.
    pub(crate) fn installing_with(status: i32) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let mise = dir.path().join("mise");
        let script = format!(
            "#!/bin/sh\n\
             [ \"$1\" = install ] && exit {status}\n\
             while [ \"$#\" -gt 0 ] && [ \"$1\" != -- ]; do shift; done\n\
             shift\n\
             exec \"$@\"\n"
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
