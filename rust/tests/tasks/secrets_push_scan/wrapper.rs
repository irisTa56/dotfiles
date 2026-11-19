//! Runs through `tasks/secrets/push-scan.sh`, the wrapper consumers run, which installs
//! the release it pins with mise and runs it.

use crate::support::mise::Mise;
use crate::support::origin::Origin;

use super::{ahead_of, leaking, main_line};

// Installing the release can fail, for a version not yet published or a host no binary
// is built for, and that is no answer about the push.
#[test]
fn fails_as_unable_to_scan_where_the_release_it_pins_cannot_be_installed() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let run = checkout.secrets_push_scan_wrapper(
        &["origin"],
        &main_line(&checkout, &sha),
        &Mise::installing_with(1),
    );
    assert_eq!((run.code(), run.output()), (2, ""));
}

// A release mise installed can still be one it cannot locate, and running a binary from
// the empty path it prints then is no answer about the push either.
#[test]
fn fails_as_unable_to_scan_where_mise_cannot_locate_the_release_it_installed() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", "fine\n")]);
    let run = checkout.secrets_push_scan_wrapper(
        &["origin"],
        &main_line(&checkout, &sha),
        &Mise::installing_but_not_locating(),
    );
    assert_eq!(run.code(), 2, "output: {}", run.output());
}

// git hands the hook the remote's name as an argument and the pushed refs on stdin, and
// the binary needs both to find what the push sends; its exit code is the push's answer.
#[test]
fn exits_as_secrets_push_scan_does_on_a_secret_the_push_sends() {
    let origin = Origin::with_main(&[("base.txt", "fine\n")]);
    let (checkout, sha) = ahead_of(&origin, &[("a.txt", &leaking())]);
    checkout
        .secrets_push_scan_wrapper(
            &["origin"],
            &main_line(&checkout, &sha),
            &Mise::installing_with(0),
        )
        .expect(1, &[&format!("Github at line 1 of a.txt, commit {sha}")]);
}
