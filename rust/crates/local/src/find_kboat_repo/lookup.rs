//! `find-kboat-repo-lookup`: prints the records of the given titles from the catalogue
//! an export left.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use super::json::Json;
use super::{CATALOG, Note, Report};
use crate::failure::Failure;

/// The fields of a record, in the order its line gives them.
const RECORD: [&str; 6] = [
    "title",
    "url",
    "summary",
    "status",
    "last_commit",
    "archived",
];

const USAGE: &str = "\
Usage: dotfiles-local find-kboat-repo-lookup <dir> <title>...

Prints, one JSON object per line and in the catalogue's order, the record of each note
whose title is given, from the catalog.json that find-kboat-repo-export-catalog wrote
into <dir>: its title, url, summary, status, last_commit and archived. A title the
catalogue does not hold prints nothing.

  --help, -h  print this message
";

/// Prints the record of each title in `args` after the export's directory, or, given
/// `--help`, prints how it is called.
pub(crate) fn run(args: &[String]) -> Result<(), Failure> {
    match args {
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        [dir, titles @ ..] => lookup(Path::new(dir), titles),
        [] => Err(Failure::new(format_args!("{USAGE}"), 2)),
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
        .map_err(|error| Failure::writing(&error))
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
