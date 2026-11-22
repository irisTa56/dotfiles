//! `dotfiles-tasks secrets-push-scan`: whether the commits a push sends hold a secret,
//! as a pre-push hook runs it. How trufflehog's report is read is checked against the real
//! trufflehog; which commits and which repository the scan reads, against a stand-in that
//! reports the commits whose changes hold a token.

mod wrapper;

use std::fs;

use crate::support::checkout::Checkout;
use crate::support::git::IDENTITY;
use crate::support::origin::Origin;
use crate::support::secret::github_token;
use crate::support::trufflehog::{Trufflehog, path_without_trufflehog};

/// The all-zero object name git gives for a ref a push creates or deletes.
const ZERO: &str = "0000000000000000000000000000000000000000";

/// A file assigning the token, as a commit might add one.
fn leaking() -> String {
    format!("token = \"{}\"\n", github_token())
}

/// The line git feeds a pre-push hook for one ref.
fn line(local_ref: &str, local_sha: &str, remote_ref: &str, remote_sha: &str) -> String {
    format!("{local_ref} {local_sha} {remote_ref} {remote_sha}\n")
}

/// A checkout of `origin`'s `main` with `files` committed on top, and the commit.
fn ahead_of(origin: &Origin, files: &[(&str, &str)]) -> (Checkout, String) {
    let checkout = Checkout::new(files);
    checkout.stage_over(origin);
    let sha = commit(&checkout, "change");
    (checkout, sha)
}

/// Commits what `checkout` stages with `message`, and returns the commit.
fn commit(checkout: &Checkout, message: &str) -> String {
    checkout.git(
        &[
            &IDENTITY[..],
            &["commit", "-q", "--allow-empty", "-m", message],
        ]
        .concat(),
    );
    checkout.git(&["rev-parse", "HEAD"])
}

/// The line for pushing `checkout`'s `main` at `sha` to `origin`'s.
fn main_line(checkout: &Checkout, sha: &str) -> String {
    let remote = checkout.git(&["rev-parse", "origin/main"]);
    line("refs/heads/main", sha, "refs/heads/main", &remote)
}

// A secret the push sends would otherwise reach the remote, and the report says where
// it sits.
#[test]
fn fails_on_a_secret_in_a_commit_the_push_sends_naming_where_it_sits() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &format!("fine\n{}", leaking()))]);
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .run()
        .expect(
            1,
            &[
                &format!("Github at line 2 of a.txt, commit {sha}"),
                "trufflehog:ignore",
            ],
        );
}

// A push that sends no secret goes through, and trufflehog's own log stays out of it.
#[test]
fn passes_a_push_that_sends_no_secret_saying_nothing() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A secret the remote already holds is not this push's to answer for, though
// trufflehog walks the whole history behind the pushed commit.
#[test]
fn passes_a_push_whose_secret_the_remote_already_holds() {
    let origin = Origin::with_main(&[("a.txt", &leaking())]);
    let (checkout, sha) = ahead_of(&origin, &[("b.txt", "fine\n")]);
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// What counts as held is what the remote the push goes to holds, not what another
// remote does.
#[test]
fn fails_on_a_secret_another_remote_holds_but_the_pushed_one_does_not() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking())]);
    let other = Origin::new();
    checkout.add_remote("other", &other);
    checkout.git(&["push", "-q", "other", "HEAD:refs/heads/main"]);
    let trufflehog = Trufflehog::reporting_tokens();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .run()
        .expect(1, &[&format!("commit {sha}")]);
}

// git runs the hook with nothing on stdin for a push that is already up to date.
#[test]
fn passes_a_push_that_sends_no_ref() {
    let origin = Origin::with_main(&[("a.txt", &leaking())]);
    let (checkout, _) = ahead_of(&origin, &[]);
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input("")
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A deletion sends no commit.
#[test]
fn passes_a_push_that_deletes_a_ref() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking())]);
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(line("(delete)", ZERO, "refs/heads/gone", &sha))
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A secret in a commit's message is sent with the commit. trufflehog reads the message
// of a commit that adds a line, and not of one that adds none.
#[test]
fn fails_on_a_secret_in_the_message_of_a_commit_the_push_sends() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let checkout = Checkout::new(&[("a.txt", "added\n")]);
    checkout.stage_over(&origin);
    let sha = commit(&checkout, &format!("add {}", github_token()));
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .run()
        .expect(1, &[&format!("of the commit message, commit {sha}")]);
}

// An annotated tag names a tag object, and a branch of the same name must not stand in
// for the commit it tags.
#[test]
fn fails_on_a_secret_in_a_commit_an_annotated_tag_sends() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking())]);
    checkout.git(&[&IDENTITY[..], &["tag", "-a", "-m", "release", "v1"]].concat());
    checkout.git(&["branch", "v1", "origin/main"]);
    let tag = checkout.git(&["rev-parse", "refs/tags/v1"]);
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(line("refs/tags/v1", &tag, "refs/tags/v1", ZERO))
        .run()
        .expect(1, &[&format!("commit {sha}")]);
}

// Each ref's secrets are reported, and how to let one through is said once.
#[test]
fn reports_the_secrets_of_every_ref_the_push_sends_and_the_way_through_once() {
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
    let trufflehog = Trufflehog::reporting_tokens();
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .arg("origin")
        .input(&refs)
        .run();
    run.expect(
        1,
        &[&format!("commit {first}"), &format!("commit {second}")],
    );
    assert_eq!(run.output().matches("trufflehog:ignore").count(), 1);
}

// A pre-push hook runs from wherever the push was started, and the repository is what it
// scans.
#[test]
fn fails_on_a_secret_the_push_sends_when_run_from_a_subdirectory() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking()), ("sub/b.txt", "fine\n")]);
    let trufflehog = Trufflehog::reporting_tokens();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .arg("origin")
        .in_dir("sub")
        .input(main_line(&checkout, &sha))
        .run()
        .expect(1, &[&format!("commit {sha}")]);
}

// Outside a repository there is no push to scan, and git says so.
#[test]
fn fails_with_gits_status_outside_a_repository() {
    let checkout = Checkout::new(&[]);
    checkout.remove(".git");
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input("")
        .run()
        .expect(128, &["not a git repository"]);
}

// git gives a pre-push hook the remote's name, and without it no commit can be told to be
// new to the remote, so the run is a usage error, which refuses the push.
#[test]
fn fails_as_a_usage_error_without_the_remote_name_git_gives_a_pre_push_hook() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .input(main_line(&checkout, &sha))
        .run()
        .expect(2, &["<REMOTE>"]);
}

// git gives a pre-push hook the remote's URL after its name, and the scan reads the name
// alone.
#[test]
fn fails_on_a_secret_the_push_sends_when_given_the_remotes_url_too() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &format!("fine\n{}", leaking()))]);
    let trufflehog = Trufflehog::reporting_tokens();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .args(["origin", "https://example.com/origin.git"])
        .input(main_line(&checkout, &sha))
        .run()
        .expect(1, &[&format!("commit {sha}")]);
}

// A scan that failed has checked nothing, and what trufflehog said is the one account of
// why.
#[test]
fn fails_with_what_trufflehog_said_where_its_scan_fails() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let bin = Trufflehog::running("echo scan broke >&2\nexit 3\n");
    let path = bin.path();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", &path)
        .run()
        .expect(1, &["scan broke"]);
}

// trufflehog comes from the task's tool header, so a consumer whose mise did not put it on
// PATH has to be told that nothing was scanned.
#[test]
fn fails_naming_trufflehog_where_it_is_not_on_path() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let path = path_without_trufflehog();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", &path)
        .run()
        .expect(1, &["trufflehog"]);
}

// git allows a ref name that is not UTF-8, and a push of one that sends no secret is
// scanned like any other rather than refused for the name.
#[test]
fn passes_a_push_of_a_ref_whose_name_is_not_utf8() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let name = b"refs/heads/caf\xe9";
    let refs = [
        &name[..],
        b" ",
        sha.as_bytes(),
        b" ",
        name,
        b" ",
        ZERO.as_bytes(),
        b"\n",
    ]
    .concat();
    let trufflehog = Trufflehog::reporting_tokens();
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .arg("origin")
        .input(&refs)
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A report whose lines no longer say which commit holds the secret cannot be told to be
// about commits the push does not send, so the push must not pass on it.
#[test]
fn fails_where_trufflehogs_report_lines_lack_the_commit() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking())]);
    let bin = Trufflehog::running("echo '{\"SourceMetadata\":{\"Data\":{}}}'\n");
    let path = bin.path();
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", &path)
        .run();
    assert_eq!(run.code(), 1, "output: {}", run.output());
}

// One ref's secret stops the push, whatever the refs after it hold.
#[test]
fn fails_on_a_secret_in_the_first_ref_the_push_sends_when_a_later_one_is_clean() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, first) = ahead_of(&origin, &[("a.txt", &leaking())]);
    checkout.git(&["switch", "-q", "-c", "topic", "origin/main"]);
    checkout.write("b.txt", "fine\n");
    checkout.git(&["add", "b.txt"]);
    let second = commit(&checkout, "topic");
    let refs = [
        main_line(&checkout, &first),
        line("refs/heads/topic", &second, "refs/heads/topic", ZERO),
    ]
    .concat();
    let trufflehog = Trufflehog::reporting_tokens();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .env("PATH", trufflehog.path())
        .arg("origin")
        .input(&refs)
        .run()
        .expect(1, &[&format!("commit {first}")]);
}

// Verifying a candidate sends it to its provider, which a push must never do; a scan that
// hit an error has not answered for every commit; and a push is no time for trufflehog to
// look for a newer release of itself. The real trufflehog shows none of these offline, so
// a stand-in records what it was asked.
#[test]
fn asks_trufflehog_not_to_verify_to_fail_on_a_scan_error_and_not_to_update() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let bin = Trufflehog::running("printf '%s\\n' \"$@\" >\"$(dirname \"$0\")/args\"\n");
    let path = bin.path();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", &path)
        .run()
        .expect(0, &[]);
    let args = fs::read_to_string(bin.dir().join("args")).expect("the stand-in ran");
    let args: Vec<&str> = args.lines().collect();
    assert!(
        ["--no-verification", "--fail-on-scan-errors", "--no-update"]
            .iter()
            .all(|flag| args.contains(flag)),
        "trufflehog was run with {args:?}"
    );
}

// A ref whose commits the remote holds already sends nothing, and a scan of it would answer
// for what the remote has, so no scan runs, and one that would fail does not stop the push.
#[test]
fn passes_a_ref_the_remote_holds_already_without_scanning_it() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, _) = ahead_of(&origin, &[]);
    let held = checkout.git(&["rev-parse", "origin/main"]);
    let bin = Trufflehog::running("echo scan broke >&2\nexit 3\n");
    let path = bin.path();
    let run = checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(line("refs/heads/other", &held, "refs/heads/other", ZERO))
        .env("PATH", &path)
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A ref line naming an object the repository lacks leaves nothing to tell new commits by,
// and git's own status stops the push rather than letting it through unscanned.
#[test]
fn fails_with_gits_status_on_a_pushed_object_the_repository_lacks() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, _) = ahead_of(&origin, &[]);
    let missing = "1".repeat(40);
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(line("refs/heads/main", &missing, "refs/heads/main", ZERO))
        .run()
        .expect(128, &["bad object"]);
}

// A report this cannot read answers for no commit, so the push stops on it.
#[test]
fn fails_where_trufflehogs_report_is_not_json() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let bin = Trufflehog::running("echo 'not json'\n");
    let path = bin.path();
    checkout
        .dotfiles_tasks()
        .arg("secrets-push-scan")
        .arg("origin")
        .input(main_line(&checkout, &sha))
        .env("PATH", &path)
        .run()
        .expect(1, &["could not read trufflehog's report"]);
}
