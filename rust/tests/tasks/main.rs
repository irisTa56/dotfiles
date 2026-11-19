//! The `dotfiles-tasks` subcommands, run as a built binary, and through the wrappers
//! consumers run, in repositories each test sets up.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod dispatch;
mod link_check;
mod skills_size_check;
mod support;
mod upstream_pin_check;
