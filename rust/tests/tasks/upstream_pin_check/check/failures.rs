//! How each end of a run without `--bump` but a pass is reported: the exit code, and
//! stdout and stderr, each whole. git is a stand-in where the end is one the real one
//! cannot be made to reach on cue.

use std::ffi::OsStr;

use regex::escape;
use tempfile::TempDir;

use crate::support::checkout::Checkout;
use crate::support::invocation::Invocation;
use crate::support::stand_in::{FAILING, KILLED, StandIns};
use crate::support::upstream::Upstream;

/// The line a run that cannot list the files to read pins from ends on.
const UNLISTED: &str = "[fail] could not list the tracked files to read pins from\n";

/// A run in a repository pinning an upstream's `lints` at its tip, finding git on `path`.
fn run_on(path: impl AsRef<OsStr>) -> (Upstream, Checkout, Invocation) {
    let up = Upstream::changing_lints(1);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    let run = checkout
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .env("PATH", path);
    (up, checkout, run)
}

#[test]
fn says_git_could_not_run_and_exits_2_where_git_is_not_on_path() {
    let bin = StandIns::new();
    let (_up, _checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        2,
        "",
        &format!("could not run git: No such file or directory (os error 2)\n{UNLISTED}"),
    );
}

#[test]
fn passes_on_what_git_said_and_exits_2_where_git_cannot_list_the_files_to_read() {
    let bin = StandIns::new().git_on("grep -lzI", FAILING);
    let (_up, _checkout, run) = run_on(bin.path());
    run.run_apart()
        .expect(2, "", &format!("stand-in: failed\n{UNLISTED}"));
}

#[test]
fn exits_2_where_a_signal_stops_git_listing_the_files_to_read() {
    let bin = StandIns::new().git_on("grep -lzI", KILLED);
    let (_up, _checkout, run) = run_on(bin.path());
    run.run_apart().expect(2, "", UNLISTED);
}

#[test]
fn passes_on_what_git_said_and_exits_2_where_git_cannot_find_the_root() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", FAILING);
    let (_up, _checkout, run) = run_on(bin.path());
    run.run_apart().expect(
        2,
        "",
        "stand-in: failed\n[fail] could not find the repository's root\n",
    );
}

#[test]
fn says_what_stopped_it_and_exits_2_where_the_root_cannot_be_entered() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let gone = aux.path().join("gone");
    let bin = StandIns::new().git_on(
        "rev-parse --show-toplevel",
        &format!("echo '{}'; exit 0", gone.display()),
    );
    let (_up, _checkout, run) = run_on(bin.path());
    run.run_apart().expect(
        2,
        "",
        "[fail] stopped on No such file or directory (os error 2)\n",
    );
}

#[test]
fn says_what_stopped_it_and_exits_2_where_a_listed_file_cannot_be_read() {
    let bin = StandIns::new().git_on("grep -lzI", "printf 'gone\\000'; exit 0");
    let (_up, _checkout, run) = run_on(bin.path());
    run.run_apart().expect(
        2,
        "",
        "[fail] stopped on No such file or directory (os error 2)\n",
    );
}

#[test]
fn says_it_found_no_pin_and_exits_2_where_no_tracked_file_spells_one() {
    Checkout::new(&[("Cargo.toml", "[package]\n")])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run_apart()
        .expect(2, "", "[fail] found no upstream pin in the tracked files\n");
}

#[test]
fn says_what_stopped_it_and_exits_2_where_it_cannot_make_its_scratch_directory() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let bin = StandIns::new();
    let (_up, _checkout, run) = run_on(bin.path());
    let gone = aux.path().join("gone");
    run.env("TMPDIR", &gone).run_apart().expect_matching(
        2,
        "",
        &format!(
            r#"\[fail\] stopped on No such file or directory \(os error 2\) at path "{}/upstream-pin-check-\w+"\n"#,
            escape(&gone.display().to_string())
        ),
    );
}

#[test]
fn passes_on_what_git_said_and_exits_2_where_git_cannot_make_the_scratch_repository() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let bin = StandIns::new().git_on("init -q", FAILING);
    let (_up, _checkout, run) = run_on(bin.path());
    run.env("TMPDIR", aux.path()).run_apart().expect_matching(
        2,
        "",
        &format!(
            r"stand-in: failed\n\[fail\] could not create a scratch repository at {}/upstream-pin-check-\w+\n",
            escape(&aux.path().display().to_string())
        ),
    );
}

// A pin that matches says so on stdout, and one that differs on stderr, each as it is
// checked; the run ends after the last.
#[test]
fn names_both_commits_of_a_pin_that_differs_and_exits_1() {
    let up = Upstream::changing_lints(2);
    let kept = Upstream::changing_lints(1);
    let cargo = format!(
        "# {}\n# {}\n",
        kept.marker("lints", 0),
        up.marker("lints", 0)
    );
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run_apart()
        .expect(
            1,
            &format!(
                "[ok] {}//lints at the pinned {c} matches the default branch ({c})\n",
                kept.url(),
                c = kept.commit(0)
            ),
            &format!(
                "[fail] {}//lints differs from the default branch, pinned in Cargo.toml\n[fail] pinned: {}\n[fail] tip:    {}\n",
                up.url(),
                up.commit(0),
                up.commit(1)
            ),
        );
}

#[test]
fn says_a_pins_path_is_gone_from_the_default_branch_and_exits_1() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("lints/a.toml", None)]]);
    Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run_apart()
        .expect(
            1,
            "",
            &format!(
                "[fail] {}//lints is gone from the default branch ({}), pinned in Cargo.toml; a bump leaves this pin where it is\n",
                up.url(),
                up.commit(1)
            ),
        );
}

// A pin it cannot tell about outweighs one that differs.
#[test]
fn exits_2_where_it_cannot_tell_about_one_pin_and_another_differs() {
    let up = Upstream::changing_lints(2);
    let cargo = format!("# {}\n# {}\n", up.marker("lint", 0), up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "[fail] {u}//lint does not exist at the pinned {c0}\n[fail] {u}//lints differs from the default branch, pinned in Cargo.toml\n[fail] pinned: {c0}\n[fail] tip:    {c1}\n",
                u = up.url(),
                c0 = up.commit(0),
                c1 = up.commit(1)
            ),
        );
}
