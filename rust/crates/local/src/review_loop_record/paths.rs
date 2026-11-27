//! `review-loop-record-paths`: prints the record's directories and files, creating the
//! directories.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

use clap::Args;
use dotfiles_shared::Failure;

use super::Record;
use crate::failure;

/// Print where the review-loop record of the branch checked out lives.
///
/// Prints the record directory, the background directory, and the branch's verdict
/// section and background files under them, one `label: path` per line, creating the
/// directories each file sits in. Run it in the checkout the loop reviews.
#[derive(Debug, Args)]
pub(crate) struct Request {}

impl Request {
    /// Creates the record's directories and prints its paths.
    #[expect(
        clippy::unused_self,
        reason = "the command takes no arguments, and every command's entry has one shape"
    )]
    pub(crate) fn run(self) -> Result<(), Failure> {
        let record = Record::locate()?;
        record.create_dirs()?;
        io::stdout()
            .write_all(record.listing().as_bytes())
            .map_err(|error| failure::writing(&error))
    }
}

impl Record {
    /// Creates the directories the two files sit in, a branch's `/` making a
    /// subdirectory.
    fn create_dirs(&self) -> Result<(), Failure> {
        for file in [self.record(), self.background()] {
            let dir = file.parent().unwrap_or(Path::new("."));
            fs::create_dir_all(dir).map_err(|error| {
                Failure::new(format_args!("cannot create {}: {error}", dir.display()), 1)
            })?;
        }
        Ok(())
    }

    /// The four paths, one `label: path` per line.
    fn listing(&self) -> String {
        let mut text = String::new();
        for (label, path) in [
            ("record directory", self.verdicts.clone()),
            ("background directory", self.backgrounds.clone()),
            ("record", self.record()),
            ("background", self.background()),
        ] {
            let _ = writeln!(text, "{label}: {}", path.display());
        }
        text
    }
}
