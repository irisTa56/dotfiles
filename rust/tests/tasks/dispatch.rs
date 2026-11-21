//! How `dotfiles-tasks` picks a subcommand, what it does with a name it has none for, and
//! how each subcommand answers `--help` and arguments it does not take.

use crate::support::checkout::{Checkout, Stream};

/// Every subcommand, by its name.
const TASKS: [&str; 4] = [
    "link-check-staged",
    "secrets-push-scan",
    "skills-size-check",
    "upstream-pin-check",
];

// A caller that reads only stdout may close stderr, and a name with no subcommand still
// has to end as a usage error.
#[test]
fn fails_as_a_usage_error_on_an_unknown_task_when_stderr_is_closed() {
    let run = Checkout::new(&[]).task_closing("frobnicate", Stream::Stderr, &[]);
    assert_eq!((run.code(), run.output()), (2, ""));
}

// A name with no subcommand, such as a misspelt one, is a usage error, said on stderr.
#[test]
fn fails_as_a_usage_error_on_an_unknown_task_naming_it_on_stderr() {
    let run = Checkout::new(&[]).task_closing("frobnicate", Stream::Stdout, &[]);
    assert_eq!(run.code(), 2, "output: {}", run.output());
    assert!(
        run.output().contains("'frobnicate'"),
        "the unknown task is not named in:\n{}",
        run.output()
    );
}

// `--help` is asked for, so it is the run's output, on stdout, and the run succeeds.
#[test]
fn help_lists_every_task_on_stdout() {
    let run = Checkout::new(&[]).task_closing("--help", Stream::Stderr, &[]);
    assert_eq!(run.code(), 0, "output: {}", run.output());
    for task in TASKS {
        assert!(
            run.output().contains(task),
            "{task} is missing from:\n{}",
            run.output()
        );
    }
}

#[test]
fn help_of_each_task_gives_its_usage_on_stdout() {
    let checkout = Checkout::new(&[]);
    for task in TASKS {
        let run = checkout.task_closing(task, Stream::Stderr, &["--help"]);
        assert_eq!(run.code(), 0, "{task} --help; output: {}", run.output());
        assert!(
            run.output()
                .contains(&format!("Usage: dotfiles-tasks {task}")),
            "{task} --help gives no usage:\n{}",
            run.output()
        );
    }
}

// An option a task does not take is a usage error, said on stderr, before the task does
// anything.
#[test]
fn each_task_fails_as_a_usage_error_on_stderr_on_an_option_it_does_not_take() {
    let checkout = Checkout::new(&[]);
    for task in TASKS {
        let run = checkout.task_closing(task, Stream::Stdout, &["--frobnicate"]);
        assert_eq!(run.code(), 2, "{task}; output: {}", run.output());
        assert!(
            run.output().contains("'--frobnicate'"),
            "{task} does not name the option:\n{}",
            run.output()
        );
    }
}

// `link-check-staged` takes no argument, so one given is not silently dropped.
#[test]
fn link_check_staged_fails_as_a_usage_error_on_an_argument() {
    Checkout::new(&[])
        .task("link-check-staged", &["README.md"], &[])
        .expect(2, &["'README.md'"]);
}
