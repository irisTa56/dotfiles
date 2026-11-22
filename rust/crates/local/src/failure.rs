//! What ends a run of this crate's commands where shared code has no say.

use std::io;

use dotfiles_shared::Failure;

/// What ends a run whose output cannot be written.
pub(crate) fn writing(error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot write the output: {error}"), 1)
}
