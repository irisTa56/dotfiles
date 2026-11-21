//! How `dotfiles-local` picks a command, what it does with a name it has none for, and
//! how every command answers `--help` and an argument it does not take.

use std::process::{Command, Output};

use crate::support::DOTFILES_LOCAL;

/// Every command, as `--help` lists them.
const COMMANDS: [&str; 14] = [
    "ask-email-in-request",
    "ccusage-popup",
    "find-kboat-repo-export-catalog",
    "find-kboat-repo-lookup",
    "fnox-add-secret",
    "git-sweep-branches",
    "git-sync-base",
    "mise-multi-upgrade",
    "push-gist-skill",
    "reply-language-stop",
    "setup-dotfiles",
    "skills-unmanaged",
    "skills-update",
    "with-new-terminal",
];

fn run(args: &[&str]) -> Output {
    Command::new(DOTFILES_LOCAL)
        .args(args)
        .output()
        .expect("dotfiles-local starts")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// The list of commands comes from where they are picked, so it names each one.
#[test]
fn lists_every_command_on_help() {
    let run = run(&["--help"]);

    assert_eq!(
        (run.status.code(), text(&run.stderr)),
        (Some(0), String::new())
    );
    let stdout = text(&run.stdout);
    let listed: Vec<&str> = stdout
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert_eq!(listed, COMMANDS, "{stdout}");
}

// A name with no command, such as a misspelt one, is a usage error, as is no name.
#[test]
fn fails_as_a_usage_error_on_an_unknown_command_or_none() {
    for args in [&["frobnicate"][..], &[]] {
        let run = run(args);
        let stderr = text(&run.stderr);
        assert_eq!(
            (run.status.code(), text(&run.stdout)),
            (Some(2), String::new()),
            "{args:?}"
        );
        assert!(
            stderr.contains("Usage: dotfiles-local <COMMAND>"),
            "{args:?}: {stderr}"
        );
    }
    assert!(
        text(&run(&["frobnicate"]).stderr)
            .starts_with("error: unrecognized subcommand 'frobnicate'"),
    );
}

// Each command prints how it is called on stdout for `--help`, and on an option it does
// not take, ends as a usage error on stderr before doing anything.
#[test]
fn every_command_answers_help_and_an_unknown_option() {
    for command in COMMANDS {
        let usage = format!("Usage: dotfiles-local {command}");

        let help = run(&[command, "--help"]);
        let stdout = text(&help.stdout);
        assert_eq!(
            (help.status.code(), text(&help.stderr)),
            (Some(0), String::new()),
            "{command}"
        );
        assert!(stdout.contains(&usage), "{command}: {stdout}");

        let wrong = run(&[command, "--no-such-option"]);
        let stderr = text(&wrong.stderr);
        assert_eq!(
            (wrong.status.code(), text(&wrong.stdout)),
            (Some(2), String::new()),
            "{command}"
        );
        assert!(
            stderr.starts_with("error: unexpected argument '--no-such-option'")
                && stderr.contains(&usage),
            "{command}: {stderr}"
        );
    }
}
