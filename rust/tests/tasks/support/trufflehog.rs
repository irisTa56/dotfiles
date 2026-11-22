//! Stand-ins for trufflehog, for what the real one cannot be made to do and for runs whose
//! question is which commits the scan reads rather than how trufflehog reads them.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A directory holding a `trufflehog` that runs a script with `sh`.
pub(crate) struct Trufflehog {
    dir: TempDir,
}

impl Trufflehog {
    /// A trufflehog that runs `script`.
    pub(crate) fn running(script: &str) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let trufflehog = dir.path().join("trufflehog");
        fs::write(&trufflehog, format!("#!/bin/sh\n{script}"))
            .expect("the stand-in can be written");
        fs::set_permissions(&trufflehog, fs::Permissions::from_mode(0o755))
            .expect("the stand-in can be made executable");
        Trufflehog { dir }
    }

    /// A trufflehog that, run as `trufflehog git <file:// URL> --branch <commit>`, reports
    /// in its `--json` form each commit in the history behind the commit whose changes add
    /// or remove a GitHub token, naming no file. Like the real one, it reads the repository
    /// through a clone of its own, so a URL that names no repository fails the scan.
    pub(crate) fn reporting_tokens() -> Self {
        Self::running(
            r#"while [ "$#" -gt 0 ]; do
    case $1 in
        file://*) url=$1 ;;
        --branch) shift; tip=$1 ;;
    esac
    shift
done
clone=$(mktemp -d) || exit 1
trap 'rm -rf "$clone"' EXIT
git clone -q --bare "$url" "$clone" || exit 1
commits=$(git -C "$clone" log --format=%H -G 'ghp_' "$tip") || exit 1
for commit in $commits; do
    printf '{"DetectorName":"Github","SourceMetadata":{"Data":{"Git":{"commit":"%s","line":1}}}}\n' "$commit"
done
"#,
        )
    }

    /// The directory the stand-in runs in, where a script may leave what it was asked.
    pub(crate) fn dir(&self) -> &Path {
        self.dir.path()
    }

    /// The tests' PATH with this stand-in ahead of the rest, and with every directory
    /// holding another `trufflehog` left out.
    pub(crate) fn path(&self) -> OsString {
        self.path_over(&inherited())
    }

    /// `path` with this stand-in ahead of the rest, and with every directory holding
    /// another `trufflehog` left out.
    pub(crate) fn path_over(&self, path: &OsStr) -> OsString {
        let dirs = std::iter::once(PathBuf::from(self.dir.path()))
            .chain(std::env::split_paths(path).filter(|dir| !dir.join("trufflehog").exists()));
        std::env::join_paths(dirs).expect("no directory on PATH holds a colon")
    }
}

/// The tests' PATH with every directory holding a `trufflehog` left out.
pub(crate) fn path_without_trufflehog() -> OsString {
    let inherited = inherited();
    let dirs = std::env::split_paths(&inherited).filter(|dir| !dir.join("trufflehog").exists());
    std::env::join_paths(dirs).expect("no directory on PATH holds a colon")
}

/// The PATH the tests run with.
fn inherited() -> OsString {
    std::env::var_os("PATH").expect("PATH is set")
}
