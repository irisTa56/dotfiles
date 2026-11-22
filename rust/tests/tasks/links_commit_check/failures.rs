//! How each end of a run but a pass is reported: the exit code, and stdout and stderr,
//! each whole. git and lychee are stand-ins where the end is one the real ones cannot be
//! made to reach on cue.

use std::ffi::OsStr;

use crate::support::checkout::Checkout;
use crate::support::invocation::Invocation;
use crate::support::stand_in::{FAILING, KILLED, StandIns};

/// What the stand-in lychee lists as the files `lychee .` reads.
const LISTING: &str = "printf './a.md\\n'\n";

/// The line a failed check adds where mise runs the task `links:commit-check`.
const SKIP: &str = "If a failing link is not one this commit adds, skip only this check: MISE_TASK_SKIP=links:commit-check.sh git commit ...\n";

/// A lychee that lists `a.md`, and runs `check` on the run that checks the links. It
/// names what it runs by path, since a run may be given a PATH holding the stand-ins
/// alone.
fn lychee_checking(check: &str) -> String {
    format!("case $1 in\n--dump-inputs) {LISTING};;\n*) /bin/cat >/dev/null\n{check};;\nesac\n")
}

/// A run of the task in a repository staging `a.md`, finding the programs it runs on
/// `path`, with no task of mise's running.
fn run_on(path: impl AsRef<OsStr>) -> (Checkout, Invocation) {
    let checkout = Checkout::new(&[("a.md", "fine\n")]);
    let run = checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("PATH", path)
        .env_remove("MISE_TASK_NAME");
    (checkout, run)
}

#[test]
fn says_git_could_not_run_and_exits_2_where_git_is_not_on_path() {
    let bin = StandIns::new();
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        2,
        "",
        "could not run git: No such file or directory (os error 2)\n",
    );
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_find_the_root() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", FAILING);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_2_where_a_signal_stops_git_finding_the_root() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", KILLED);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(2, "", "");
}

#[test]
fn says_lychee_could_not_run_and_exits_127_where_lychee_is_not_on_path() {
    let bin = StandIns::new().git_on("never given", "");
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        127,
        "",
        "could not run lychee: No such file or directory (os error 2)\n",
    );
}

#[test]
fn says_lychee_could_not_run_and_exits_126_where_lychee_is_not_executable() {
    let bin = StandIns::new()
        .git_on("never given", "")
        .unrunnable("lychee");
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        126,
        "",
        "could not run lychee: Permission denied (os error 13)\n",
    );
}

#[test]
fn exits_with_lychees_status_where_lychee_cannot_list_the_files() {
    let bin = StandIns::new().program("lychee", FAILING);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_2_where_a_signal_stops_lychee_listing_the_files() {
    let bin = StandIns::new().program("lychee", KILLED);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(2, "", "");
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_list_the_staged_files() {
    let bin = StandIns::new()
        .program("lychee", &lychee_checking("exit 0"))
        .git_on("diff --cached", FAILING);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_2_where_a_signal_stops_git_listing_the_staged_files() {
    let bin = StandIns::new()
        .program("lychee", &lychee_checking("exit 0"))
        .git_on("diff --cached", KILLED);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(2, "", "");
}

// The stand-in git removes itself once it has found the root, so the run that lists the
// staged files finds no git.
#[test]
fn says_git_could_not_run_and_exits_2_where_git_is_gone_by_the_listing_of_staged_files() {
    let bin = StandIns::new()
        .program("lychee", &lychee_checking("exit 0"))
        .git_on("rev-parse --show-toplevel", "/bin/rm \"$0\"");
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        2,
        "",
        "could not run git: No such file or directory (os error 2)\n",
    );
}

#[test]
fn exits_1_where_lychee_finds_a_broken_link_naming_how_to_skip_the_task_mise_runs() {
    let bin = StandIns::new().program(
        "lychee",
        &lychee_checking("echo 'stand-in: a.md: broken'\nexit 2"),
    );
    let (_checkout, run) = run_on(bin.path());
    run.env("MISE_TASK_NAME", "links:commit-check")
        .run_apart()
        .expect(1, "stand-in: a.md: broken\n", SKIP);
}

#[test]
fn exits_1_where_lychee_finds_a_broken_link_naming_no_skip_outside_mise() {
    let bin = StandIns::new().program(
        "lychee",
        &lychee_checking("echo 'stand-in: a.md: broken'\nexit 2"),
    );
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(1, "stand-in: a.md: broken\n", "");
}

// lychee's own statuses for a run it could not finish, such as 3 for a config it cannot
// load, end the run as a broken link does.
#[test]
fn exits_1_where_lychee_fails_checking_the_links_with_a_status_of_its_own() {
    let bin = StandIns::new().program("lychee", &lychee_checking(FAILING));
    let (_checkout, run) = run_on(bin.path());
    run.env("MISE_TASK_NAME", "links:commit-check")
        .run_apart()
        .expect(1, "", &format!("stand-in: failed\n{SKIP}"));
}

#[test]
fn exits_1_where_a_signal_stops_lychee_checking_the_links() {
    let bin = StandIns::new().program("lychee", &lychee_checking(KILLED));
    let (_checkout, run) = run_on(bin.path());
    run.env("MISE_TASK_NAME", "links:commit-check")
        .run_apart()
        .expect(1, "", SKIP);
}

// The stand-in lychee removes itself once it has listed the files, so the run that
// checks the links finds no lychee.
#[test]
fn says_lychee_could_not_run_and_exits_127_where_lychee_is_gone_by_the_check() {
    let bin = StandIns::new()
        .git_on("never given", "")
        .program("lychee", &format!("{LISTING}/bin/rm \"$0\"\n"));
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        127,
        "",
        "could not run lychee: No such file or directory (os error 2)\n",
    );
}
