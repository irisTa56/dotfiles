//! `find-kboat-repo-export-catalog`: exports the catalogue through kboat-note into a
//! fresh temporary directory and splits a compact copy of it into chunks for judging.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;
use serde_json::value::RawValue;

use super::{CATALOG, Json, Note, Report};
use crate::failure::{Failure, exit_code};

/// The fields of a note kboat-note is asked for: what judging and lookup read.
const FIELDS: [&str; 10] = [
    "title",
    "url",
    "description",
    "topics",
    "summary",
    "domain",
    "role",
    "archived",
    "last_commit",
    "status",
];

/// The fields of a note a judge reads, in the order a compact line gives them.
const JUDGED: [&str; 6] = [
    "title",
    "role",
    "domain",
    "topics",
    "description",
    "summary",
];

/// The judged fields kboat-note may give as a flow-style list in a string.
const LISTS: [&str; 2] = ["domain", "topics"];

/// The notes in one chunk, which one judging subagent reads.
const CHUNK: usize = 100;

const USAGE: &str = "\
Usage: dotfiles-local find-kboat-repo-export-catalog

Exports the K-Boat repo catalogue through `kboat-note list`, run in the k-boat checkout
KBOAT_REPO names, into a fresh temporary directory, and splits a compact copy of it into
chunk files of 100 notes, one for each judging subagent of the find-kboat-repo skill.
Prints the directory, the note count, the anomalies kboat-note reported and the chunk
files as JSON. Where kboat-note fails, prints the directory, its exit code and its
anomalies instead, and exits with that code.

  --help, -h  print this message
";

/// Exports the catalogue and prints where it went, or, given `--help`, prints how it is
/// called.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args {
        [] => export(),
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

fn export() -> Result<(), Failure> {
    let kboat_repo = kboat_repo()?;
    let dir = fresh_dir()?;
    let text = list(&kboat_repo, &dir)?;
    let report: Report<'_> = serde_json::from_str(&text).map_err(|error| {
        Failure::new(
            format_args!(
                "cannot read kboat-note's report in {}: {error}",
                dir.join(CATALOG).display()
            ),
            1,
        )
    })?;
    let chunks = report.write_chunks(&dir)?;
    print(&report.summary(&dir, &chunks))
}

/// The k-boat checkout `KBOAT_REPO` names.
fn kboat_repo() -> Result<OsString, Failure> {
    env::var_os("KBOAT_REPO")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            Failure::new(
                format_args!(
                    "KBOAT_REPO is not set: it names this machine's k-boat checkout (dotfiles README, Setting up K-Boat)."
                ),
                2,
            )
        })
}

/// A new directory under TMPDIR, named as `mktemp -d` names one, which outlives the run
/// so that the judges and lookup can read it.
fn fresh_dir() -> Result<PathBuf, Failure> {
    tempfile::Builder::new()
        .prefix("tmp.")
        .rand_bytes(10)
        .tempdir()
        .map(tempfile::TempDir::keep)
        .map_err(|error| Failure::new(format_args!("cannot create a directory: {error}"), 1))
}

/// Runs `kboat-note list` in the k-boat checkout with its stdout in `dir`'s catalog.json,
/// and returns what it wrote.
fn list(kboat_repo: &OsStr, dir: &Path) -> Result<String, Failure> {
    let path = dir.join(CATALOG);
    let file = File::create(&path).map_err(|error| {
        Failure::new(format_args!("cannot write {}: {error}", path.display()), 1)
    })?;
    let mut command = Command::new("mise");
    command
        .arg("-C")
        .arg(kboat_repo)
        .args(["x", "--", "kboat-note", "list", "--type", "repo"]);
    for field in FIELDS {
        command.args(["--field", field]);
    }
    let status = command
        .stdout(file)
        .status()
        .map_err(|error| Failure::new(format_args!("cannot run mise: {error}"), 1))?;
    let text = fs::read(&path).map_err(|error| {
        Failure::new(format_args!("cannot read {}: {error}", path.display()), 1)
    })?;
    let text = String::from_utf8_lossy(&text).into_owned();
    if !status.success() {
        print(&failed(dir, exit_code(status), &text))?;
        return Err(Failure::passed_on(status));
    }
    Ok(text)
}

impl Report<'_> {
    /// Writes the compact copy of the notes into `dir`, and splits it into chunk files,
    /// whose paths it returns.
    fn write_chunks(&self, dir: &Path) -> Result<Vec<PathBuf>, Failure> {
        let catalog = dir.join(CATALOG);
        let lines = self
            .notes
            .iter()
            .map(|note| {
                note.compact_line()
                    .map_err(|error| note.unreadable(&catalog, error))
            })
            .collect::<Result<Vec<String>, Failure>>()?;
        write(&dir.join("compact.jsonl"), &lines)?;
        lines
            .chunks(CHUNK)
            .enumerate()
            .map(|(index, lines)| {
                let path = dir.join(format!("chunk-{}", suffix(index)?));
                write(&path, lines)?;
                Ok(path)
            })
            .collect()
    }

    /// What a run that read the catalogue prints: the directory, the note count, the
    /// anomalies kboat-note reported, and the chunk files.
    fn summary(&self, dir: &Path, chunks: &[PathBuf]) -> String {
        let chunks: Vec<String> = chunks.iter().map(|path| string(path)).collect();
        format!(
            "{{\"dir\":{},\"notes\":{},\"anomalies\":{},\"chunks\":[{}]}}",
            string(dir),
            self.notes.len(),
            self.anomalies.map_or("null", RawValue::get),
            chunks.join(",")
        )
    }
}

impl Note<'_> {
    /// The note as one line of JSON holding only what judging reads.
    fn compact_line(&self) -> Result<String, String> {
        let members = JUDGED
            .iter()
            .map(|name| Ok(format!("\"{name}\":{}", self.judged(name)?)))
            .collect::<Result<Vec<String>, String>>()?;
        Ok(format!("{{{}}}", members.join(",")))
    }

    /// The field `name` as a judge reads it, as compact JSON.
    fn judged(&self, name: &str) -> Result<String, String> {
        let Some(value) = self.field(name) else {
            return Ok(if LISTS.contains(&name) { "[]" } else { "null" }.to_owned());
        };
        if LISTS.contains(&name) {
            return as_list(name, value);
        }
        Json(value.get())
            .compact()
            .map_err(|error| error.to_string())
    }
}

/// The list field `name`, given as `value`, as a JSON array: kboat-note returns a
/// flow-style list such as `[robotics, ml]` as a string, and an empty one as an array.
fn as_list(name: &str, value: &RawValue) -> Result<String, String> {
    match serde_json::from_str(value.get()).map_err(|error| error.to_string())? {
        serde_json::Value::Array(_) => Json(value.get())
            .compact()
            .map_err(|error| error.to_string()),
        serde_json::Value::Null => Ok("[]".to_owned()),
        serde_json::Value::String(text) => {
            let text = text.strip_prefix('[').unwrap_or(&text);
            let text = text.strip_suffix(']').unwrap_or(text);
            let items: Vec<&str> = text
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .collect();
            Ok(serde_json::Value::from(items).to_string())
        }
        other => Err(format!("{name} is {other}, neither a list nor a string")),
    }
}

/// The suffix `split` gives the chunk at `index`: `aa`, `ab`, and on to `zz`.
fn suffix(index: usize) -> Result<String, Failure> {
    let letter = |n: usize| char::from(b'a' + u8::try_from(n).expect("n is below 26"));
    if index >= 26 * 26 {
        return Err(Failure::new(
            format_args!("the catalogue needs more than {} chunks", 26 * 26),
            1,
        ));
    }
    Ok(format!("{}{}", letter(index / 26), letter(index % 26)))
}

/// Writes `lines` into the file at `path`, each ended by a newline.
fn write(path: &Path, lines: &[String]) -> Result<(), Failure> {
    let text: String = lines
        .iter()
        .flat_map(|line| [line.as_str(), "\n"])
        .collect();
    fs::write(path, text)
        .map_err(|error| Failure::new(format_args!("cannot write {}: {error}", path.display()), 1))
}

/// What a failed kboat-note left of its report: its exit 1 still carries one, whose
/// anomalies say why Repos/ could not be read.
#[derive(Deserialize)]
struct Remains<'a> {
    #[serde(borrow, default)]
    anomalies: Option<&'a RawValue>,
}

/// What a run whose kboat-note failed with `code` prints: the directory, the code, and
/// the anomalies of the report kboat-note left in `text`, or null where it left none.
fn failed(dir: &Path, code: u8, text: &str) -> String {
    let anomalies = serde_json::from_str::<Remains<'_>>(text)
        .ok()
        .and_then(|remains| remains.anomalies)
        .map_or("null", RawValue::get);
    format!(
        "{{\"dir\":{},\"exit\":{code},\"anomalies\":{anomalies}}}",
        string(dir)
    )
}

/// `path` as a JSON string.
fn string(path: &Path) -> String {
    serde_json::Value::from(path.to_string_lossy()).to_string()
}

/// Prints the JSON text `json`, indented by two spaces.
fn print(json: &str) -> Result<(), Failure> {
    let text = Json(json)
        .pretty()
        .map_err(|error| Failure::new(format_args!("cannot lay out the output: {error}"), 1))?;
    io::stdout()
        .write_all(text.as_bytes())
        .map_err(|error| Failure::writing(&error))
}
