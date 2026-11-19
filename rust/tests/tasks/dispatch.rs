//! How `dotfiles-tasks` picks a subcommand, and what it does with a name it has none for.

use crate::support::checkout::{Checkout, Stream};

// A caller that reads only stdout may close stderr, and a name with no subcommand still
// has to end as a run that could not tell.
#[test]
fn fails_as_unable_to_tell_on_an_unknown_task_when_stderr_is_closed() {
    let run = Checkout::new(&[]).task_closing("frobnicate", Stream::Stderr, &[]);
    assert_eq!((run.code(), run.output()), (2, ""));
}

// A name with no subcommand, such as a misspelt one, is told which tasks there are.
#[test]
fn fails_as_unable_to_tell_on_an_unknown_task_naming_the_tasks() {
    let run = Checkout::new(&[]).task("frobnicate", &[], &[]);
    assert_eq!(
        (run.code(), run.output()),
        (
            2,
            "[fail] usage: dotfiles-tasks <task> [args...]; tasks: link-check-staged, secrets-push-scan, skills-size-check, upstream-pin-check\n"
        )
    );
}

// tasks/upstream/pin-check.sh called the pin check as pin-check while pinning 0.4.0.
#[test]
fn runs_upstream_pin_check_under_its_old_name_pin_check() {
    let run = Checkout::new(&[("Cargo.toml", "[package]\n")]).task("pin-check", &[], &[]);
    assert_eq!(
        (run.code(), run.output()),
        (2, "[fail] found no upstream pin in the tracked files\n")
    );
}

// tasks/skills/size-check.sh called the size check as size-check while pinning 0.4.0.
#[test]
fn runs_skills_size_check_under_its_old_name_size_check() {
    Checkout::new(&[])
        .task("size-check", &[], &[])
        .expect(1, &["git tracks no skill"]);
}
