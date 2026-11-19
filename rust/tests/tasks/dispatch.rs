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
            "[fail] usage: dotfiles-tasks <task> [args...]; tasks: link-check-staged, skills-size-check, upstream-pin-check\n"
        )
    );
}

// The wrappers of releases that named the subcommands without their namespace still call
// pin-check, so it runs the pin check.
#[test]
fn runs_upstream_pin_check_under_its_old_name_pin_check() {
    let run = Checkout::new(&[("Cargo.toml", "[package]\n")]).task("pin-check", &[], &[]);
    assert_eq!(
        (run.code(), run.output()),
        (2, "[fail] found no upstream pin in the tracked files\n")
    );
}

// The wrappers of releases that named the subcommands without their namespace still call
// size-check, so it runs the size check.
#[test]
fn runs_skills_size_check_under_its_old_name_size_check() {
    Checkout::new(&[])
        .task("size-check", &[], &[])
        .expect(1, &["git tracks no skill"]);
}
