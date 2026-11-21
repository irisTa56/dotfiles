//! The temporary directories, whose configs belong to trials, an agent's scratchpad
//! among them, and not to anything worth keeping current.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// Each temporary directory, under the name it is given and the one it resolves to,
/// since mise reports a config by the resolved path: `/tmp` is a symlink on macOS.
#[derive(Debug)]
pub(super) struct TempDirs {
    dirs: Vec<String>,
}

impl TempDirs {
    /// The system's temporary directory, as `system_temp_dir` picks it, and `/tmp`
    /// whatever that is.
    pub(super) fn find() -> Self {
        let cwd = std::env::current_dir().ok();
        let picked = system_temp_dir(cwd.as_deref());
        let mut dirs: Vec<String> = Vec::new();
        for dir in picked.iter().chain([&PathBuf::from("/tmp")]) {
            let names = [expand(dir, cwd.as_deref()), fs::canonicalize(dir).ok()];
            for name in names.into_iter().flatten() {
                let name = name.to_string_lossy().into_owned();
                if !dirs.contains(&name) {
                    dirs.push(name);
                }
            }
        }
        TempDirs { dirs }
    }

    /// Whether `dir` is a temporary directory or lies under one.
    pub(super) fn contain(&self, dir: &str) -> bool {
        self.dirs.iter().any(|temp| {
            dir.strip_prefix(temp.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        })
    }
}

/// The first of `TMPDIR`, `TEMP`, `TMP`, `/tmp` and `cwd` that names a directory that can
/// be written, if any.
fn system_temp_dir(cwd: Option<&Path>) -> Option<PathBuf> {
    ["TMPDIR", "TEMP", "TMP"]
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(PathBuf::from))
        .chain([PathBuf::from("/tmp")])
        .chain(cwd.map(Path::to_path_buf))
        .find(|dir| writable_dir(dir))
}

/// Whether `dir` is a directory whose permissions let something be written in it.
fn writable_dir(dir: &Path) -> bool {
    fs::metadata(dir).is_ok_and(|metadata| metadata.is_dir() && !metadata.permissions().readonly())
}

/// `dir` made absolute from `cwd` and with its `.` and `..` steps taken by name, without
/// following a symlink, or `None` where it is relative and there is no `cwd`.
fn expand(dir: &Path, cwd: Option<&Path>) -> Option<PathBuf> {
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        cwd?.join(dir)
    };
    let mut expanded = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                expanded.pop();
            }
            other => expanded.push(other),
        }
    }
    Some(expanded)
}
