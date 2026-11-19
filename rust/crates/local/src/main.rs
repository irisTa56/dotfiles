//! The commands for this machine alone, one subcommand each, which `mise run
//! local:install` installs into `~/.local/bin` from the main checkout.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    // A name with no command ends as dotfiles-tasks ends on one, unable to tell.
    let _ = writeln!(
        io::stderr(),
        "[fail] usage: dotfiles-local <command> [args...]"
    );
    ExitCode::from(2)
}
