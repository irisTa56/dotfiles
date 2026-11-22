//! The `dotfiles-tasks` subcommands, run as a built binary, and through the wrappers
//! consumers run, in repositories each test sets up; `secrets:commit-scan`, a shell
//! task with no subcommand, run as its script; and that each wrapper pins the release
//! this package is at.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod dispatch;
mod links_commit_check;
mod release_pins;
mod secrets_commit_scan;
mod secrets_push_scan;
mod skills_size_check;
mod support;
mod upstream_pin_check;
