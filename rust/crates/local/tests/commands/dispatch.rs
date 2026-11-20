//! How `dotfiles-local` picks a command, and what it does with a name it has none for.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
        .args(args)
        .output()
        .expect("dotfiles-local starts")
}

// A name with no command, such as a misspelt one, is told how the binary is called.
#[test]
fn fails_with_usage_on_an_unknown_command() {
    let run = run(&["frobnicate"]);
    assert_eq!(
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stderr).as_ref()
        ),
        (
            Some(2),
            "[fail] usage: dotfiles-local <command> [args...]; commands: ask-email-in-request, block-bsd-sed-inplace, block-bsd-stat-format, block-piped-git-push, ccusage-popup, find-kboat-repo-export-catalog, find-kboat-repo-lookup, git-sweep-branches, git-sync-base, mise-multi-upgrade, reply-language-stop, skills-unmanaged, skills-update, with-new-terminal\n"
        )
    );
}
