//! The find-kboat-repo skill's commands: `find-kboat-repo-export-catalog` exports the
//! K-Boat repo catalogue for judging, and `find-kboat-repo-lookup` reads back the records
//! of the titles judged to fit.
//!
//! Values are read as raw JSON rather than parsed into maps, so an object's keys keep
//! the order kboat-note wrote them in, as they did through jq.

pub(crate) mod export_catalog;
pub(crate) mod lookup;

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::Path;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::failure::Failure;

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

impl Note<'_> {
    /// The frontmatter field `name`, if the note has it.
    fn field(&self, name: &str) -> Option<&RawValue> {
        self.frontmatter.as_ref()?.get(name).copied()
    }

    /// What ends a run on a note of the report in `catalog` that cannot be read for
    /// `error`, naming both, so the note can be found among many.
    fn unreadable(&self, catalog: &Path, error: impl fmt::Display) -> Failure {
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

/// JSON text, laid out again with its keys in the order the text has them.
struct Json<'a>(&'a str);

impl Json<'_> {
    /// The text with no whitespace between tokens.
    fn compact(&self) -> serde_json::Result<String> {
        let mut out = Vec::new();
        self.transcode(&mut serde_json::Serializer::new(&mut out))?;
        Ok(String::from_utf8(out).expect("serde_json writes UTF-8"))
    }

    /// The text indented by two spaces and ended by a newline.
    fn pretty(&self) -> serde_json::Result<String> {
        let mut out = Vec::new();
        self.transcode(&mut serde_json::Serializer::pretty(&mut out))?;
        out.push(b'\n');
        Ok(String::from_utf8(out).expect("serde_json writes UTF-8"))
    }

    fn transcode<W: io::Write, F: serde_json::ser::Formatter>(
        &self,
        serializer: &mut serde_json::Serializer<W, F>,
    ) -> serde_json::Result<()> {
        let mut deserializer = serde_json::Deserializer::from_str(self.0);
        serde_transcode::transcode(&mut deserializer, serializer)?;
        deserializer.end()
    }
}
