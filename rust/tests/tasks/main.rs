//! The `dotfiles-tasks` subcommands, run as a built binary, and through the wrappers
//! consumers run, in repositories each test sets up.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod pin_check;
mod size_check;
mod support;
