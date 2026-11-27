//! `review-loop-record-selector-input`: writes the close's selector's input into a new
//! file and prints its path.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use clap::Args;
use dotfiles_shared::Failure;

use super::Record;
use crate::failure;

/// Write what the review-loop close's selector reads into a new file, and print its path.
///
/// Writes the background, the verdict section and the finding-bar skill whole, each
/// under a `===== <path> =====` line naming the file it came from, into a new file in the
/// temporary directory, outside the record's directories, and prints that file's path.
/// Fails, writing nothing, where any of the three is missing or empty. Run it in the
/// checkout the loop reviews, just before spawning the selector.
#[derive(Debug, Args)]
pub(crate) struct Request {}

impl Request {
    /// Writes the selector's input and prints where.
    #[expect(
        clippy::unused_self,
        reason = "the command takes no arguments, and every command's entry has one shape"
    )]
    pub(crate) fn run(self) -> Result<(), Failure> {
        let record = Record::locate()?;
        let input = record.selector_input()?;
        let written = write(&input)?;
        let mut line = written.into_os_string().into_encoded_bytes();
        line.push(b'\n');
        io::stdout()
            .write_all(&line)
            .map_err(|error| failure::writing(&error))
    }
}

impl Record {
    /// The finding-bar skill, which the selector weighs each item against.
    fn finding_bar(&self) -> PathBuf {
        self.home.join(".claude/skills/finding-bar/SKILL.md")
    }

    /// The background, the verdict section and finding-bar, each under a line naming
    /// its path.
    fn selector_input(&self) -> Result<Vec<u8>, Failure> {
        let mut input = Vec::new();
        for path in [self.background(), self.record(), self.finding_bar()] {
            input.extend_from_slice(format!("===== {} =====\n\n", path.display()).as_bytes());
            input.extend(read(&path)?);
            input.push(b'\n');
        }
        Ok(input)
    }
}

/// What `path` holds, failing where it is empty.
fn read(path: &Path) -> Result<Vec<u8>, Failure> {
    let text = fs::read(path).map_err(|error| {
        Failure::new(format_args!("cannot read {}: {error}", path.display()), 1)
    })?;
    if text.is_empty() {
        return Err(Failure::new(format_args!("{} is empty", path.display()), 1));
    }
    Ok(text)
}

/// Writes `input` into a new file in the temporary directory, kept once written.
fn write(input: &[u8]) -> Result<PathBuf, Failure> {
    let failing = |error: io::Error| {
        Failure::new(
            format_args!("cannot write the selector's input: {error}"),
            1,
        )
    };
    let mut file = tempfile::Builder::new()
        .prefix("review-loop-selector.")
        .suffix(".md")
        .tempfile()
        .map_err(failing)?;
    file.write_all(input).map_err(failing)?;
    let (_, path) = file.keep().map_err(|error| failing(error.error))?;
    Ok(path)
}
