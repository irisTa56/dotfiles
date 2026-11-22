//! `git-sweep-branches`: deletes the local branches origin's default branch holds whole,
//! after gh-poi has deleted the ones a merged pull request holds. It runs the real git
//! against an origin and a clone of the test's own, with fakes standing in for gh-poi and
//! gh, and with nothing else on PATH.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, IDENTITY, Sandbox, git, isolated};

/// A bare origin whose default branch is `main`, a clone of it to run in, and a second
/// clone that moves origin on, none of which the machine's git config reaches.
struct Fixture {
    sandbox: Sandbox,
}

impl Fixture {
    /// A fresh fixture named after `test`, where origin's `main` holds one commit, the
    /// work clone has `current` checked out at it, and gh-poi and gh succeed doing nothing.
    fn new(test: &str) -> Self {
        let fixture = Fixture {
            sandbox: Sandbox::new("git-sweep-branches", test, &[]),
        };
        fixture
            .sandbox
            .fake("git", &format!("exec '{}' \"$@\"", git().display()));
        fixture.gh_poi("");
        fixture
            .sandbox
            .fake("gh", "echo gh >> \"$SANDBOX/calls\"; exit 1");
        fixture.git(
            fixture.root(),
            &[
                "init",
                "--quiet",
                "--bare",
                "--initial-branch=main",
                "origin.git",
            ],
        );
        fixture.git(fixture.root(), &["init", "--quiet", "upstream"]);
        fixture.upstream(&["remote", "add", "origin", "../origin.git"]);
        fixture.commit("first");
        fixture.upstream(&["push", "--quiet", "origin", "HEAD:main"]);
        fixture.git(fixture.root(), &["clone", "--quiet", "origin.git", "work"]);
        fixture.work(&["switch", "--quiet", "--create", "current"]);
        fixture
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// Makes gh-poi record its call and then run the shell script `body`.
    fn gh_poi(&self, body: &str) {
        self.sandbox.fake(
            "gh-poi",
            &format!("echo gh-poi >> \"$SANDBOX/calls\"\n{body}"),
        );
    }

    /// The commands among the fakes that were called, one a line, in order.
    fn calls(&self) -> String {
        fs::read_to_string(self.root().join("calls")).unwrap_or_default()
    }

    /// Makes a commit in the upstream clone.
    fn commit(&self, message: &str) {
        self.upstream(&["commit", "--quiet", "--allow-empty", "--message", message]);
    }

    /// Runs git with `args` in the upstream clone.
    fn upstream(&self, args: &[&str]) {
        self.git(&self.root().join("upstream"), args);
    }

    /// Runs git with `args` in the work clone.
    fn work(&self, args: &[&str]) {
        self.git(&self.root().join("work"), args);
    }

    fn git(&self, dir: &Path, args: &[&str]) {
        let status = self
            .command(git(), dir)
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// The local branches of the work clone, one a line.
    fn branches(&self) -> String {
        let output = self
            .command(git(), &self.root().join("work"))
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
        let mut command = isolated(program, dir, self.root());
        command
            .env("PATH", self.sandbox.bin())
            .env("SANDBOX", self.root())
            .envs(IDENTITY);
        command
    }

    /// Runs `git-sweep-branches` with `args` in the work clone.
    fn run(&self, args: &[&str]) -> Output {
        self.command(DOTFILES_LOCAL, &self.root().join("work"))
            .arg("git-sweep-branches")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

// After gh-poi, each local branch origin's default branch holds whole is deleted, with
// what `git branch -D` prints; a branch with a commit of its own stays.
#[test]
fn deletes_the_branches_the_default_branch_holds() {
    let sandbox = Fixture::new("merged");
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
    let sandbox = Fixture::new("locked");
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
    let sandbox = Fixture::new("order");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:gone"]);
    sandbox.work(&["fetch", "--quiet", "origin"]);
    sandbox.upstream(&["push", "--quiet", "origin", "--delete", "gone"]);
    sandbox.work(&["worktree", "add", "--quiet", "../stale", "-b", "stale"]);
    fs::remove_dir_all(sandbox.root().join("stale")).expect("the worktree is removed");
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
    let sandbox = Fixture::new("held");
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
    let sandbox = Fixture::new("default-changed");
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:trunk"]);
    sandbox.git(
        &sandbox.root().join("origin.git"),
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
    let sandbox = Fixture::new("gh-poi-fails");
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
    let sandbox = Fixture::new("fetch-fails");
    sandbox.work(&["branch", "merged"]);
    fs::rename(
        sandbox.root().join("origin.git"),
        sandbox.root().join("gone.git"),
    )
    .expect("origin is moved away");

    let run = sandbox.run(&[]);

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(128), "{stderr}");
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.branches(), "current\nmain\nmerged\n");
}

// `--help` prints how it is called and touches nothing; any other argument is a usage
// error on stderr with exit 2, before anything is fetched or deleted.
#[test]
fn prints_usage_on_help_and_fails_as_a_usage_error_on_any_argument() {
    let sandbox = Fixture::new("usage");
    sandbox.work(&["branch", "merged"]);

    let help = sandbox.run(&["--help"]);
    let short = sandbox.run(&["-h"]);
    let mistakes = [&["merged"][..], &["--dry-run"]].map(|args| sandbox.run(args));

    for run in [help, short] {
        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
        assert_eq!(run.status.code(), Some(0));
        assert!(
            stdout.contains("Usage: dotfiles-local git-sweep-branches\n"),
            "{stdout}"
        );
    }
    for mistake in mistakes {
        let stderr = String::from_utf8_lossy(&mistake.stderr).into_owned();
        assert_eq!(mistake.status.code(), Some(2), "{stderr}");
        assert!(
            stderr.starts_with("error: unexpected argument")
                && stderr.contains("Usage: dotfiles-local git-sweep-branches\n"),
            "{stderr}"
        );
    }
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.branches(), "current\nmain\nmerged\n");
}
