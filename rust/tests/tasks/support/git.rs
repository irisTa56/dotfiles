//! git as the tests run it: kept to the repository it is run in.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// A command that runs `program` in `dir` with git kept to the repository there: a git
/// hook running the tests exports `GIT_DIR` and its kin for its own repository, and a
/// directory above the temporary one could be a repository. It applies to git run by a
/// test and by the binary alike.
pub(super) fn isolated(program: impl AsRef<OsStr>, dir: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(dir);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.env(
        "GIT_CEILING_DIRECTORIES",
        dir.parent().expect("a temporary directory has a parent"),
    );
    command
}

/// The git the tests' own setup and reads run: the program in git's exec path, which the
/// `git` on PATH may only reach through a shim, such as the one macOS puts in `/usr/bin`
/// that looks up the developer tools on every run. Where the exec path holds no `git`,
/// the one on PATH.
fn program() -> &'static Path {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(|| {
        let output = Command::new("git")
            .arg("--exec-path")
            .output()
            .expect("git runs");
        let exec_path = String::from_utf8(output.stdout).expect("git's exec path is UTF-8");
        let git = Path::new(exec_path.trim_end()).join("git");
        if output.status.success() && git.is_file() {
            git
        } else {
            PathBuf::from("git")
        }
    })
}

/// Runs git in `dir` to read what a run left there, returning its stdout as git wrote
/// it, or `None` where git fails, as on an object that does not exist.
pub(super) fn query(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = isolated(program(), dir)
        .args(args)
        .output()
        .expect("git runs");
    output.status.success().then_some(output.stdout)
}

/// Runs git in `dir`, as setup that must succeed, and returns its trimmed stdout.
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let output = isolated(program(), dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {dir:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git's output is UTF-8")
        .trim()
        .to_owned()
}

/// The git identity setup commits are made with, since the host may have none.
pub(crate) const IDENTITY: [&str; 4] = ["-c", "user.name=t", "-c", "user.email=t@example.com"];
