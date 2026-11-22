//! `git-sync-base`: brings a base branch up to date with origin, run with the real git
//! against an origin, clones and worktrees of the test's own, which it acts on nothing
//! outside of.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, IDENTITY, Sandbox, git, isolated};

/// A bare origin whose default branch is `main`, a clone of it to run in, and a second
/// clone that moves origin on, none of which the machine's git config reaches.
struct Fixture {
    sandbox: Sandbox,
}

impl Fixture {
    /// A fresh fixture named after `test`, where origin's `main` and `feature` hold one
    /// commit each and the work clone has `feature` checked out.
    fn new(test: &str) -> Self {
        let fixture = Fixture {
            sandbox: Sandbox::new("git-sync-base", test, &[]),
        };
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
        fixture.upstream(&["push", "--quiet", "origin", "HEAD:main", "HEAD:feature"]);
        fixture.git(fixture.root(), &["clone", "--quiet", "origin.git", "work"]);
        fixture.git(&fixture.work(), &["switch", "--quiet", "feature"]);
        fixture
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    fn work(&self) -> PathBuf {
        self.root().join("work")
    }

    /// Makes a commit in the upstream clone.
    fn commit(&self, message: &str) {
        self.upstream(&["commit", "--quiet", "--allow-empty", "--message", message]);
    }

    /// Runs git with `args` in the upstream clone.
    fn upstream(&self, args: &[&str]) {
        self.git(&self.root().join("upstream"), args);
    }

    fn git(&self, dir: &Path, args: &[&str]) {
        let status = self
            .command(git(), dir)
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// What git with `args` prints in `dir`, trimmed.
    fn read(&self, dir: &Path, args: &[&str]) -> String {
        let output = self
            .command(git(), dir)
            .args(args)
            .output()
            .expect("git starts");
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// `program` run in `dir`, with git kept to the fixture and a fixed committer.
    fn command(&self, program: impl AsRef<OsStr>, dir: &Path) -> Command {
        let mut command = isolated(program, dir, self.root());
        command.envs(IDENTITY);
        command
    }

    /// Runs `git-sync-base` with `args` in `dir`.
    fn run(&self, dir: &Path, args: &[&str]) -> Output {
        self.command(DOTFILES_LOCAL, dir)
            .arg("git-sync-base")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

// With no branch named, the base is origin's default branch: the checkout switches to
// it and fast-forwards to what origin has.
#[test]
fn switches_to_origins_default_branch_and_fast_forwards() {
    let sandbox = Fixture::new("default-branch");
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);

    let run = sandbox.run(&sandbox.work(), &[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        (
            sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
            sandbox.read(&sandbox.work(), &["log", "-1", "--format=%s"]),
        ),
        ("main".to_owned(), "second".to_owned())
    );
}

// The branch named is the base, with or without the `origin/` a remote-tracking name
// starts with.
#[test]
fn takes_the_branch_named_as_the_base() {
    for (test, name) in [
        ("named-branch", "release"),
        ("named-remote-branch", "origin/release"),
    ] {
        let sandbox = Fixture::new(test);
        sandbox.commit("release");
        sandbox.upstream(&["push", "--quiet", "origin", "HEAD:release"]);

        let run = sandbox.run(&sandbox.work(), &[name]);

        assert_eq!(
            run.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(
            sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
            "release",
            "{name}"
        );
    }
}

// Where another worktree has the base checked out, git refuses to switch to it, so that
// worktree is fast-forwarded in place and this checkout stays where it is, saying so.
#[test]
fn fast_forwards_the_worktree_holding_the_base_and_stays() {
    let sandbox = Fixture::new("held-elsewhere");
    let holder = sandbox.root().join("held 日本");
    sandbox.git(
        &sandbox.work(),
        &[
            "worktree",
            "add",
            "--quiet",
            holder.to_str().expect("UTF-8"),
            "main",
        ],
    );
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);

    let run = sandbox.run(&sandbox.work(), &["main"]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stderr).ends_with(&format!(
            "Fast-forwarded main in {}; this checkout stays on feature.\n",
            fs::canonicalize(&holder)
                .expect("the holder exists")
                .display()
        )),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        (
            sandbox.read(&holder, &["log", "-1", "--format=%s"]),
            sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
        ),
        ("second".to_owned(), "feature".to_owned())
    );
}

// A detached checkout has no branch to name, so the message says what it stays on.
#[test]
fn names_a_detached_head_it_stays_on() {
    let sandbox = Fixture::new("held-elsewhere-detached");
    let holder = sandbox.root().join("held");
    sandbox.git(
        &sandbox.work(),
        &["worktree", "add", "--quiet", "../held", "main"],
    );
    sandbox.git(&sandbox.work(), &["switch", "--quiet", "--detach"]);

    let run = sandbox.run(&sandbox.work(), &["main"]);

    assert_eq!(
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stderr).into_owned()
        ),
        (
            Some(0),
            format!(
                "Fast-forwarded main in {}; this checkout stays on a detached HEAD.\n",
                fs::canonicalize(&holder)
                    .expect("the holder exists")
                    .display()
            )
        )
    );
}

// Where this checkout has the base checked out itself, it fast-forwards in place, as
// the switch to the branch it is on does nothing.
#[test]
fn fast_forwards_the_base_this_checkout_is_on() {
    let sandbox = Fixture::new("on-the-base");
    sandbox.git(&sandbox.work(), &["switch", "--quiet", "main"]);
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);

    let run = sandbox.run(&sandbox.work(), &[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        sandbox.read(&sandbox.work(), &["log", "-1", "--format=%s"]),
        "second"
    );
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("Already on 'main'"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

// Where git fails, the run ends with git's exit code and what git says alone, and
// does nothing after it: a base that has moved apart from origin is not switched to
// and left unmerged.
#[test]
fn ends_with_gits_failure() {
    let sandbox = Fixture::new("diverged");
    sandbox.commit("upstream");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);
    sandbox.git(&sandbox.work(), &["switch", "--quiet", "main"]);
    sandbox.git(
        &sandbox.work(),
        &["commit", "--quiet", "--allow-empty", "--message", "local"],
    );

    let run = sandbox.run(&sandbox.work(), &["main"]);

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(128), "{stderr}");
    assert!(
        stderr.ends_with("fatal: Not possible to fast-forward, aborting.\n"),
        "{stderr}"
    );
    assert_eq!(
        sandbox.read(&sandbox.work(), &["log", "-1", "--format=%s"]),
        "local"
    );
}

// A fetch that fails stops the run before anything is switched.
#[test]
fn stops_where_the_fetch_fails() {
    let sandbox = Fixture::new("fetch-fails");
    fs::rename(
        sandbox.root().join("origin.git"),
        sandbox.root().join("gone.git"),
    )
    .expect("origin is moved away");

    let run = sandbox.run(&sandbox.work(), &["main"]);

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(128), "{stderr}");
    assert!(stderr.ends_with("fatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists.\n"), "{stderr}");
    assert_eq!(
        sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
        "feature"
    );
}

// The fast-forward of main fires the post-merge hook, and the switch to it the
// post-checkout hook, each of which reinstalls dotfiles-local by putting a new file in
// its place while this run is still going. The run carries on from the file it started
// from and ends as it would have otherwise.
#[test]
fn finishes_after_a_hook_replaces_its_binary() {
    let sandbox = Fixture::new("replaced-binary");
    let binary = sandbox.root().join("dotfiles-local");
    fs::copy(DOTFILES_LOCAL, &binary).expect("the binary is copied");
    let reinstall = format!(
        "#!/bin/sh\ncp '{0}' '{0}.new' && mv '{0}.new' '{0}'\n",
        binary.display()
    );
    for hook in ["post-checkout", "post-merge"] {
        let path = sandbox.work().join(".git/hooks").join(hook);
        fs::write(&path, &reinstall).expect("the hook is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the hook is made executable");
    }
    let before = fs::metadata(&binary).expect("the binary exists").ino();
    sandbox.commit("second");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:main"]);

    let run = sandbox
        .command(&binary, &sandbox.work())
        .arg("git-sync-base")
        .output()
        .expect("the copied binary starts");

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_ne!(
        fs::metadata(&binary).expect("the binary exists").ino(),
        before
    );
    assert_eq!(
        (
            sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
            sandbox.read(&sandbox.work(), &["log", "-1", "--format=%s"]),
        ),
        ("main".to_owned(), "second".to_owned())
    );
}

// Origin's default branch is read afresh on each run, so a default changed since the
// clone is the one taken.
#[test]
fn follows_a_default_branch_origin_changed() {
    let sandbox = Fixture::new("default-changed");
    sandbox.upstream(&["push", "--quiet", "origin", "HEAD:trunk"]);
    sandbox.git(
        &sandbox.root().join("origin.git"),
        &["symbolic-ref", "HEAD", "refs/heads/trunk"],
    );

    let run = sandbox.run(&sandbox.work(), &[]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
        "trunk"
    );
}

// An empty name is no name, as `git-sync-base ""` was to the shell it replaces.
#[test]
fn takes_an_empty_name_as_none() {
    let sandbox = Fixture::new("empty-name");

    let run = sandbox.run(&sandbox.work(), &[""]);

    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
        "main"
    );
}

// `--help` prints how it is called and touches nothing; more than one branch is a usage
// error, before anything is fetched.
#[test]
fn prints_usage_on_help_and_fails_as_a_usage_error_on_more_than_one_branch() {
    let sandbox = Fixture::new("usage");
    fs::rename(
        sandbox.root().join("origin.git"),
        sandbox.root().join("gone.git"),
    )
    .expect("origin is moved away");

    let help = sandbox.run(&sandbox.work(), &["--help"]);
    let two = sandbox.run(&sandbox.work(), &["main", "feature"]);

    let help_out = String::from_utf8_lossy(&help.stdout);
    assert_eq!(help.status.code(), Some(0));
    assert!(
        help_out.contains("Usage: dotfiles-local git-sync-base [BRANCH]\n"),
        "{help_out}"
    );
    let two_err = String::from_utf8_lossy(&two.stderr);
    assert_eq!(two.status.code(), Some(2), "{two_err}");
    assert!(
        two_err.starts_with("error: unexpected argument 'feature'")
            && two_err.contains("Usage: dotfiles-local git-sync-base [BRANCH]\n"),
        "{two_err}"
    );
}

// An argument starting with `-` is a mistake in how it is called, not an option for
// git, even after `--`: it is a usage error on stderr with exit 2, before anything is
// fetched or switched, so the checkout stays on the branch it was on.
#[test]
fn takes_a_dash_argument_as_a_usage_mistake() {
    let sandbox = Fixture::new("dash-argument");

    for args in [&["--detach"][..], &["--", "--detach"]] {
        let run = sandbox.run(&sandbox.work(), args);

        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.starts_with("error: "), "{args:?}: {stderr}");
    }
    assert_eq!(
        sandbox.read(&sandbox.work(), &["branch", "--show-current"]),
        "feature"
    );
}
