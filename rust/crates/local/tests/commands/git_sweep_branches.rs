//! `git-sweep-branches`: deletes the local branches origin's default branch holds whole,
//! after gh-poi has deleted the ones a merged pull request holds. It runs the real git
//! against an origin and a clone of the test's own, with fakes standing in for gh-poi and
//! gh, and with nothing else on PATH.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A bare origin whose default branch is `main`, a clone of it to run in, and a second
/// clone that moves origin on, none of which the machine's git config reaches.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// A fresh sandbox named after `test`, where origin's `main` holds one commit, the
    /// work clone has `current` checked out at it, and gh-poi and gh succeed doing nothing.
    fn new(test: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("git-sweep-branches")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).expect("the sandbox is created");
        let sandbox = Sandbox { root };
        sandbox.fake("git", &format!("exec '{}' \"$@\"", real("git").display()));
        sandbox.gh_poi("");
        sandbox.fake("gh", "echo gh >> \"$SANDBOX/calls\"; exit 1");
        sandbox.git(
            &sandbox.root,
            &[
                "init",
                "--quiet",
                "--bare",
                "--initial-branch=main",
                "origin.git",
            ],
        );
        sandbox.git(&sandbox.root, &["init", "--quiet", "upstream"]);
        sandbox.upstream(&["remote", "add", "origin", "../origin.git"]);
        sandbox.commit("first");
        sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);
        sandbox.git(&sandbox.root, &["clone", "--quiet", "origin.git", "work"]);
        sandbox.work(&["switch", "--quiet", "--create", "current"]);
        sandbox
    }

    /// Writes a fake `name` into the sandbox's PATH, running the shell script `body`.
    fn fake(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
    }

    /// Makes gh-poi record its call and then run the shell script `body`.
    fn gh_poi(&self, body: &str) {
        self.fake(
            "gh-poi",
            &format!("echo gh-poi >> \"$SANDBOX/calls\"\n{body}"),
        );
    }

    /// The commands among the fakes that were called, one a line, in order.
    fn calls(&self) -> String {
        fs::read_to_string(self.root.join("calls")).unwrap_or_default()
    }

    /// Makes a commit in the upstream clone.
    fn commit(&self, message: &str) {
        self.upstream(&["commit", "--quiet", "--allow-empty", "--message", message]);
    }

    /// Runs git with `args` in the upstream clone.
    fn upstream(&self, args: &[&str]) {
        self.git(&self.root.join("upstream"), args);
    }

    /// Runs git with `args` in the work clone.
    fn work(&self, args: &[&str]) {
        self.git(&self.root.join("work"), args);
    }

    fn git(&self, dir: &Path, args: &[&str]) {
        let status = self
            .command("git", dir)
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// The local branches of the work clone, one a line.
    fn branches(&self) -> String {
        let output = self
            .command("git", &self.root.join("work"))
            .args(["for-each-ref", "--format=%(refname:short)", "refs/heads"])
            .output()
            .expect("git starts");
        assert!(output.status.success(), "git for-each-ref failed");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// `program` run in `dir` with the sandbox's PATH alone, with what a git hook sets
    /// in the environment taken away, the machine's git config left unread, and a fixed
    /// committer.
    fn command(&self, program: impl AsRef<std::ffi::OsStr>, dir: &Path) -> Command {
        let mut command = Command::new(program);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        command
            .current_dir(dir)
            .env("PATH", self.root.join("bin"))
            .env("SANDBOX", &self.root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.root)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com");
        command
    }

    /// Runs `git-sweep-branches` with `args` in the work clone.
    fn run(&self, args: &[&str]) -> Output {
        self.command(
            env!("CARGO_BIN_EXE_dotfiles-local"),
            &self.root.join("work"),
        )
        .arg("git-sweep-branches")
        .args(args)
        .output()
        .expect("dotfiles-local starts")
    }
}

/// The path of `program` on the PATH the tests run with.
fn real(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set"))
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} is on PATH"))
}

// After gh-poi, each local branch origin's default branch holds whole is deleted, with
// what `git branch -D` prints; a branch with a commit of its own stays.
#[test]
fn deletes_the_branches_the_default_branch_holds() {
    let sandbox = Sandbox::new("merged");
    sandbox.work(&["branch", "merged"]);
    sandbox.work(&["switch", "--quiet", "--create", "own"]);
    sandbox.work(&["commit", "--quiet", "--allow-empty", "--message", "own"]);
    sandbox.work(&["switch", "--quiet", "current"]);

    let run = sandbox.run(&[]);

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        stdout.starts_with("Deleted branch merged (was "),
        "{stdout}"
    );
    assert_eq!(sandbox.branches(), "current\nmain\nown\n");
    assert_eq!(sandbox.calls(), "gh-poi\n");
}

// A branch `gh-poi lock` holds stays, its lock read as git reads a boolean; one whose
// lock git reads as false, or cannot read as a boolean at all, goes.
#[test]
fn keeps_a_branch_gh_poi_locks() {
    let sandbox = Sandbox::new("locked");
    for (branch, lock) in [
        ("locked", "true"),
        ("locked-yes", "yes"),
        ("unlocked", "false"),
        ("unreadable", "maybe"),
    ] {
        sandbox.work(&["branch", branch]);
        sandbox.work(&["config", &format!("branch.{branch}.gh-poi-locked"), lock]);
    }

    let run = sandbox.run(&[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(sandbox.branches(), "current\nlocked\nlocked-yes\nmain\n");
}

// gh-poi runs once origin is fetched, with the remote-tracking branches origin no longer
// has pruned, and once the worktrees whose directories are gone are pruned, so it sees
// neither; and it runs before the sweep, so it sees every merged branch still there.
// A branch it deletes is not looked for again.
#[test]
fn runs_gh_poi_after_the_fetch_and_prune_and_before_the_sweep() {
    let sandbox = Sandbox::new("order");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:gone"]);
    sandbox.work(&["fetch", "--quiet", "origin"]);
    sandbox.upstream(&["push", "--quiet", "origin", "--delete", "gone"]);
    sandbox.work(&["worktree", "add", "--quiet", "../stale", "-b", "stale"]);
    fs::remove_dir_all(sandbox.root.join("stale")).expect("the worktree is removed");
    sandbox.work(&["branch", "merged"]);
    sandbox.work(&["branch", "pulled"]);
    sandbox.gh_poi(concat!(
        "git for-each-ref --format='%(refname)' refs/remotes refs/heads >> \"$SANDBOX/calls\"\n",
        "git worktree list --porcelain | while read -r key path; do\n",
        "  [ \"$key\" = worktree ] && echo \"${path##*/}\" >> \"$SANDBOX/calls\"\n",
        "done\n",
        "git branch --quiet -D pulled",
    ));

    let run = sandbox.run(&[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        sandbox.calls(),
        concat!(
            "gh-poi\n",
            "refs/heads/current\n",
            "refs/heads/main\n",
            "refs/heads/merged\n",
            "refs/heads/pulled\n",
            "refs/heads/stale\n",
            "refs/remotes/origin/HEAD\n",
            "refs/remotes/origin/main\n",
            "work\n",
        )
    );
    assert_eq!(sandbox.branches(), "current\nmain\n");
}

// A branch checked out in another worktree stays, as git cannot delete it.
#[test]
fn keeps_a_branch_another_worktree_has_checked_out() {
    let sandbox = Sandbox::new("held");
    sandbox.work(&["worktree", "add", "--quiet", "../held 日本", "-b", "held"]);

    let run = sandbox.run(&[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(sandbox.branches(), "current\nheld\nmain\n");
}

// Origin's default branch is read afresh on each run, so the branches swept are the ones
// a default changed since the clone holds, and that default itself stays.
#[test]
fn follows_a_default_branch_origin_changed() {
    let sandbox = Sandbox::new("default-changed");
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:trunk"]);
    sandbox.git(
        &sandbox.root.join("origin.git"),
        &["symbolic-ref", "HEAD", "refs/heads/trunk"],
    );
    sandbox.work(&["fetch", "--quiet", "origin", "trunk:trunk"]);
    sandbox.work(&["switch", "--quiet", "--create", "on-trunk", "trunk"]);
    sandbox.work(&["switch", "--quiet", "current"]);

    let run = sandbox.run(&[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(sandbox.branches(), "current\ntrunk\n");
}

// Where gh-poi fails, the run ends with its exit code and deletes nothing.
#[test]
fn stops_where_gh_poi_fails() {
    let sandbox = Sandbox::new("gh-poi-fails");
    sandbox.work(&["branch", "merged"]);
    sandbox.gh_poi("echo 'gh-poi failed' >&2; exit 3");

    let run = sandbox.run(&[]);

    assert_eq!(
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stderr).into_owned()
        ),
        (Some(3), "gh-poi failed\n".to_owned())
    );
    assert_eq!(sandbox.branches(), "current\nmain\nmerged\n");
}

// A fetch that fails ends the run with git's exit code before gh-poi runs.
#[test]
fn stops_where_the_fetch_fails() {
    let sandbox = Sandbox::new("fetch-fails");
    sandbox.work(&["branch", "merged"]);
    fs::rename(
        sandbox.root.join("origin.git"),
        sandbox.root.join("gone.git"),
    )
    .expect("origin is moved away");

    let run = sandbox.run(&[]);

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(128), "{stderr}");
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.branches(), "current\nmain\nmerged\n");
}

// `--help` prints how it is called and touches nothing; any other argument is a mistake
// told the same way on stderr with exit 2, before anything is fetched or deleted.
#[test]
fn prints_usage_on_help_and_on_any_argument() {
    let sandbox = Sandbox::new("usage");
    sandbox.work(&["branch", "merged"]);

    let help = sandbox.run(&["--help"]);
    let short = sandbox.run(&["-h"]);
    let mistakes =
        [&["merged"][..], &["--dry-run"], &["--help", "x"]].map(|args| sandbox.run(args));

    let help_out = String::from_utf8_lossy(&help.stdout).into_owned();
    assert_eq!(help.status.code(), Some(0));
    assert!(
        help_out.starts_with("Usage: git-sweep-branches\n"),
        "{help_out}"
    );
    assert_eq!(
        (short.status.code(), String::from_utf8_lossy(&short.stdout)),
        (Some(0), help_out.as_str().into())
    );
    for mistake in mistakes {
        assert_eq!(
            (
                mistake.status.code(),
                String::from_utf8_lossy(&mistake.stderr).into_owned()
            ),
            (Some(2), help_out.clone())
        );
    }
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.branches(), "current\nmain\nmerged\n");
}
