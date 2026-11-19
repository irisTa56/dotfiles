//! The `dotfiles-local` subcommands, run as a built binary.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod dispatch;
mod with_new_terminal;
