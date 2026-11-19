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
            "[fail] usage: dotfiles-tasks <task> [args...]; tasks: link-check-staged, pin-check, size-check\n"
        )
    );
}
