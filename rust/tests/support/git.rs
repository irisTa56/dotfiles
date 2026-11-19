//! git as the tests run it: kept to the repository it is run in.

use std::path::Path;
use std::process::Command;

/// Keeps git, run by a test or by the binary, to the repository it is run in: a git hook
/// running the tests exports `GIT_DIR` and its kin for its own repository, and a
/// directory above the temporary one could be a repository.
pub(super) fn isolate(command: &mut Command, dir: &Path) {
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.env(
        "GIT_CEILING_DIRECTORIES",
        dir.parent().expect("a temporary directory has a parent"),
    );
}

/// Runs git in `dir`, as setup that must succeed, and returns its trimmed stdout.
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    isolate(&mut command, dir);
    let output = command.output().expect("git runs");
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
