//! How each end of a run but a pass is reported: the exit code, and stdout and stderr,
//! each whole. git and trufflehog are stand-ins where the end is one the real ones cannot
//! be made to reach on cue.

use std::ffi::OsStr;

use tempfile::TempDir;

use super::{ZERO, ahead_of, commit, leaking, line, main_line};
use crate::support::checkout::Checkout;
use crate::support::invocation::Invocation;
use crate::support::origin::Origin;
use crate::support::stand_in::{FAILING, KILLED, StandIns};
use crate::support::trufflehog::{Trufflehog, path_without_trufflehog};

/// The line said once after a push's hits.
const WAY_THROUGH: &str = "A trufflehog:ignore comment on a reported line lets it through only when the commit that introduced the line carries it, so adding one means rewriting that commit; it lets a real secret through as readily, so an agent pushing for someone shows them these hits and rewrites only on their confirmation.\n";

/// A push of one commit holding `contents` in `a.txt` over origin's `main`, run as the
/// hook, finding the programs it runs on `path`; and the commit.
fn push_on(contents: &str, path: impl AsRef<OsStr>) -> (Origin, Checkout, Invocation, String) {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", contents)]);
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", path);
    (origin, checkout, run, sha)
}

#[test]
fn says_git_could_not_run_and_exits_1_where_git_is_not_on_path() {
    let bin = StandIns::new();
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.dir());
    run.run_apart().expect(
        1,
        "",
        "could not run git: No such file or directory (os error 2)\n",
    );
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_find_the_root() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", FAILING);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_1_where_a_signal_stops_git_finding_the_root() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", KILLED);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(1, "", "");
}

// A directory given as stdin opens, and every read of it fails.
#[test]
fn says_it_could_not_read_the_pushed_refs_and_exits_1_where_stdin_cannot_be_read() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let bin = StandIns::new();
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.stdin_from(dir.path()).run_apart().expect(
        1,
        "",
        "could not read the pushed refs: Is a directory (os error 21)\n",
    );
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_list_the_commits_sent() {
    let bin = StandIns::new().git_on("rev-list", FAILING);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_1_where_a_signal_stops_git_listing_the_commits_sent() {
    let bin = StandIns::new().git_on("rev-list", KILLED);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(1, "", "");
}

// The stand-in git removes itself once it has found the root, so the run that lists the
// commits finds no git.
#[test]
fn says_git_could_not_run_and_exits_1_where_git_is_gone_by_the_listing_of_commits() {
    let bin = StandIns::new().git_on("rev-parse --show-toplevel", "/bin/rm \"$0\"");
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.dir());
    run.run_apart().expect(
        1,
        "",
        "could not run git: No such file or directory (os error 2)\n",
    );
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_peel_the_pushed_object() {
    let bin = StandIns::new().git_on("^{commit}", FAILING);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_1_where_a_signal_stops_git_peeling_the_pushed_object() {
    let bin = StandIns::new().git_on("^{commit}", KILLED);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(1, "", "");
}

#[test]
fn says_trufflehog_could_not_run_and_exits_1_where_it_is_not_on_path() {
    let (_origin, _checkout, run, _) = push_on("fine\n", path_without_trufflehog());
    run.run_apart().expect(
        1,
        "",
        "could not run trufflehog: No such file or directory (os error 2)\n",
    );
}

#[test]
fn passes_on_what_trufflehog_said_and_exits_1_where_its_scan_fails() {
    let bin = StandIns::new().program("trufflehog", FAILING);
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(1, "", "stand-in: failed\n");
}

#[test]
fn passes_on_what_trufflehog_said_and_exits_1_where_a_signal_stops_its_scan() {
    let bin = StandIns::new().program("trufflehog", &format!("echo 'stand-in: log' >&2\n{KILLED}"));
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(1, "", "stand-in: log\n");
}

#[test]
fn passes_on_trufflehogs_log_and_says_why_where_its_report_is_not_json() {
    let bin = StandIns::new().program("trufflehog", "echo 'stand-in: log' >&2\necho 'not json'\n");
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    // The reason after the colon is serde_json's, worded as that crate words it.
    run.run_apart().expect_matching(
        1,
        "",
        r"stand-in: log\ncould not read trufflehog's report: [^\n]+\n",
    );
}

#[test]
fn passes_on_trufflehogs_log_and_says_why_where_a_result_names_no_commit() {
    let bin = StandIns::new().program(
        "trufflehog",
        "echo 'stand-in: log' >&2\necho '{\"SourceMetadata\":{\"Data\":{}}}'\n",
    );
    let (_origin, _checkout, run, _) = push_on("fine\n", bin.path());
    run.run_apart().expect(
        1,
        "",
        "stand-in: log\ncould not read trufflehog's report: a result names no commit\n",
    );
}

#[test]
fn names_each_hit_and_the_way_through_once_and_exits_1_on_a_secret_the_push_sends() {
    let trufflehog = Trufflehog::reporting_tokens();
    let (_origin, _checkout, run, sha) = push_on(&leaking(), trufflehog.path());
    run.run_apart().expect(
        1,
        "",
        &format!("Github at line 1 of the commit message, commit {sha}\n{WAY_THROUGH}"),
    );
}

/// A push of two refs: `main`, one commit over origin's holding a token in `a.txt`, and a
/// new `topic`, one commit over origin's `main` holding a token in `b.txt`; with the two
/// commits and the lines git feeds the hook.
fn two_refs() -> (Origin, Checkout, String, String, String) {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, first) = ahead_of(&origin, &[("a.txt", &leaking())]);
    checkout.git(&["switch", "-q", "-c", "topic", "origin/main"]);
    checkout.write("b.txt", leaking());
    checkout.git(&["add", "b.txt"]);
    let second = commit(&checkout, "topic");
    let refs = [
        main_line(&checkout, &first),
        line("refs/heads/topic", &second, "refs/heads/topic", ZERO),
    ]
    .concat();
    (origin, checkout, first, second, refs)
}

/// The line a hit trufflehog's stand-in reports in `commit` gets.
fn hit(commit: &str) -> String {
    format!("Github at line 1 of the commit message, commit {commit}\n")
}

#[test]
fn names_each_refs_hits_in_turn_and_the_way_through_once_after_the_last() {
    let (_origin, checkout, first, second, refs) = two_refs();
    let trufflehog = Trufflehog::reporting_tokens();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(&refs)
        .env("PATH", trufflehog.path())
        .run_apart()
        .expect(
            1,
            "",
            &format!("{}{}{WAY_THROUGH}", hit(&first), hit(&second)),
        );
}

// The hits of a ref scanned before the run stopped are said as they were found, and the
// way through, said after the last ref, is not.
#[test]
fn keeps_an_earlier_refs_hits_and_says_no_way_through_where_a_later_ref_stops_the_run() {
    let (_origin, checkout, first, second, refs) = two_refs();
    let trufflehog = Trufflehog::reporting_tokens();
    let bin = StandIns::new().git_on(&format!("rev-list {second}"), FAILING);
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(&refs)
        .env("PATH", bin.path_over(&trufflehog.path()))
        .run_apart()
        .expect(5, "", &format!("{}stand-in: failed\n", hit(&first)));
}
