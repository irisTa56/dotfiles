//! Reading one note of kboat-note's report.

use std::fmt;
use std::path::Path;

use dotfiles_shared::Failure;
use serde_json::value::RawValue;

use super::Note;

impl Note<'_> {
    /// The frontmatter field `name`, if the note has it.
    pub(super) fn field(&self, name: &str) -> Option<&RawValue> {
        self.frontmatter.as_ref()?.get(name).copied()
    }

    /// What ends a run on a note of the report in `catalog` that cannot be read for
    /// `error`, naming both, so the note can be found among many.
    pub(super) fn unreadable(&self, catalog: &Path, error: impl fmt::Display) -> Failure {
        Failure::new(
            format_args!(
                "cannot read note {} in {}: {error}",
                self.path.as_deref().unwrap_or("(no path)"),
                catalog.display()
            ),
            1,
        )
    }
}
