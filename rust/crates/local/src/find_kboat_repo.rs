//! The find-kboat-repo skill's commands: `find-kboat-repo-export-catalog` exports the
//! K-Boat repo catalogue for judging, and `find-kboat-repo-lookup` reads back the records
//! of the titles judged to fit.
//!
//! Values are read as raw JSON rather than parsed into maps, so an object's keys keep
//! the order kboat-note wrote them in.

pub(crate) mod export_catalog;
mod json;
pub(crate) mod lookup;
mod note;

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::value::RawValue;

/// The file in the export's directory holding kboat-note's report whole.
const CATALOG: &str = "catalog.json";

/// The report `kboat-note list` prints, as far as these commands read it.
#[derive(Deserialize)]
struct Report<'a> {
    #[serde(borrow)]
    notes: Vec<Note<'a>>,
    /// The notes kboat-note could not read, or read with a field missing or misstated.
    #[serde(borrow, default)]
    anomalies: Option<&'a RawValue>,
}

/// One repo note of the report.
#[derive(Deserialize)]
struct Note<'a> {
    /// Where the note sits in the vault, such as `Repos/0130878e2cb8.md`.
    #[serde(default)]
    path: Option<String>,
    #[serde(borrow, default)]
    frontmatter: Option<HashMap<String, &'a RawValue>>,
}
