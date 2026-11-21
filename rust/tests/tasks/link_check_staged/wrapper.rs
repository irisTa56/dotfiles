//! Runs through `tasks/link-check/staged.sh`, the wrapper consumers run, which installs
//! the release it pins with mise and runs it.

use crate::support::mise::{Mise, POSTINSTALL};

use super::staged;

// Installing the release can fail, for a version not yet published or a host no binary
// is built for, and that is no answer about the links.
#[test]
fn fails_as_unable_to_check_where_the_release_it_pins_cannot_be_installed() {
    let run = staged(&[("a.md", "[gone](missing.md)\n")])
        .link_check_staged_wrapper(&Mise::installing_with(1));
    assert_eq!((run.code(), run.output()), (2, ""));
}

// A release mise installed can still be one it cannot locate, and running a binary from
// the empty path it prints then is no answer about the links either.
#[test]
fn fails_as_unable_to_check_where_mise_cannot_locate_the_release_it_installed() {
    let run = staged(&[("a.md", "[gone](missing.md)\n")])
        .link_check_staged_wrapper(&Mise::installing_but_not_locating());
    assert_eq!(run.code(), 2, "output: {}", run.output());
}

// The binary's exit code is the commit's answer, so the wrapper must end with it.
#[test]
fn exits_as_link_check_staged_does_on_a_broken_link() {
    staged(&[("a.md", "[gone](missing.md)\n")])
        .link_check_staged_wrapper(&Mise::installing_with(0))
        .expect(
            1,
            &[
                "missing.md",
                "MISE_TASK_SKIP=link-check:staged.sh git commit",
            ],
        );
}

// mise runs a consumer's `[hooks] postinstall` on every install in its project;
// installing the release is the wrapper's business, not an occasion for the consumer's
// setup.
#[test]
fn installs_the_release_without_running_the_consumers_postinstall() {
    let run = staged(&[("a.md", "no link\n")]).link_check_staged_wrapper(&Mise::installing_with(0));
    assert_eq!(run.code(), 0, "{}", run.output());
    assert!(!run.output().contains(POSTINSTALL), "{}", run.output());
}
