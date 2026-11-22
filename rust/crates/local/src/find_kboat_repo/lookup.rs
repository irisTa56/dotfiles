//! `find-kboat-repo-lookup`: prints the records of the given titles from the catalogue
//! an export left.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use dotfiles_shared::Failure;

use super::json::Json;
use super::{CATALOG, Note, Report};
use crate::failure;

/// The fields of a record, in the order its line gives them.
const RECORD: [&str; 6] = [
    "title",
    "url",
    "summary",
    "status",
    "last_commit",
    "archived",
];

/// Print the records of the given titles from the catalogue an export left.
///
/// Prints, one JSON object per line and in the catalogue's order, the record of each note
/// whose title is given, from the catalog.json that find-kboat-repo-export-catalog wrote
/// into DIR: its title, url, summary, status, `last_commit` and archived. A title the
/// catalogue does not hold prints nothing.
#[derive(Debug, Args)]
pub(crate) struct Request {
    /// The directory the export wrote
    dir: PathBuf,
    /// The titles whose records to print
    titles: Vec<String>,
}

impl Request {
    /// Prints the record of each title the request names.
    pub(crate) fn run(self) -> Result<(), Failure> {
        lookup(&self.dir, &self.titles)
    }
}

fn lookup(dir: &Path, titles: &[String]) -> Result<(), Failure> {
    let path = dir.join(CATALOG);
    let text = fs::read_to_string(&path).map_err(|error| {
        Failure::new(format_args!("cannot read {}: {error}", path.display()), 1)
    })?;
    let report: Report<'_> = serde_json::from_str(&text).map_err(|error| {
        Failure::new(format_args!("cannot read {}: {error}", path.display()), 1)
    })?;
    let lines = report
        .notes
        .iter()
        .filter(|note| note.titled(titles))
        .map(|note| note.record().map_err(|error| note.unreadable(&path, error)))
        .collect::<Result<Vec<String>, Failure>>()?;
    let text: String = lines
        .iter()
        .flat_map(|line| [line.as_str(), "\n"])
        .collect();
    io::stdout()
        .write_all(text.as_bytes())
        .map_err(|error| failure::writing(&error))
}

impl Note<'_> {
    /// Whether the note's title is one of `titles`.
    fn titled(&self, titles: &[String]) -> bool {
        self.field("title")
            .and_then(|title| serde_json::from_str::<String>(title.get()).ok())
            .is_some_and(|title| titles.contains(&title))
    }

    /// The note's record as one line of JSON.
    fn record(&self) -> serde_json::Result<String> {
        let members = RECORD
            .iter()
            .map(|name| {
                let value = match self.field(name) {
                    Some(value) => Json::new(value.get()).compact()?,
                    None => "null".to_owned(),
                };
                Ok(format!("\"{name}\":{value}"))
            })
            .collect::<serde_json::Result<Vec<String>>>()?;
        Ok(format!("{{{}}}", members.join(",")))
    }
}
