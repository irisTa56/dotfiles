//! `dotfiles-tasks pin-check`, run as a built binary in repositories each test sets up.
//!
//! The tests are one crate, so that a helper no test uses any longer is reported as
//! dead code.

mod bump;
mod check;
mod support;
