//! The commands for this machine alone, one subcommand each, which `mise run
//! local:install` installs into `~/.local/bin` from the main checkout.

mod ccusage_popup;
mod failure;
mod skills_update;
mod with_new_terminal;

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, rest)) if command == "ccusage-popup" => ccusage_popup::run(rest),
        Some((command, rest)) if command == "skills-update" => skills_update::run(rest),
        Some((command, rest)) if command == "with-new-terminal" => with_new_terminal::run(rest),
        _ => {
            // A name with no command ends as dotfiles-tasks ends on one, unable to tell.
            let _ = writeln!(
                io::stderr(),
                "[fail] usage: dotfiles-local <command> [args...]; commands: ccusage-popup, skills-update, with-new-terminal"
            );
            ExitCode::from(2)
        }
    }
}
