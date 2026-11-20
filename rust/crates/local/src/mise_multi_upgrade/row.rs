//! A tool a table shows, from what `mise outdated` reports of it.

use serde_json::{Map, Value};

/// A tool `mise outdated` reports in a place, with the versions a table shows.
#[derive(Debug)]
pub(super) struct Row {
    tool: String,
    requested: Option<String>,
    current: Option<String>,
    latest: Option<String>,
    bump_latest: Option<String>,
}

/// What `mise outdated --json` reports: an entry per tool, keyed by its name.
pub(super) type Entries = Map<String, Value>;

impl Row {
    /// The rows of the tools in `pinned`, what `mise outdated` reports, or `bumped`, what
    /// `mise outdated --bump` does, each once and ordered by name, or the name of the
    /// tool whose entry lacks a key mise always reports.
    ///
    /// A tool in `bumped` alone is newer only past its request, and its latest version
    /// within the request is the one it has.
    pub(super) fn merge(pinned: &Entries, bumped: &Entries) -> Result<Vec<Row>, String> {
        let mut tools: Vec<&String> = pinned.keys().chain(bumped.keys()).collect();
        tools.sort();
        tools.dedup();
        tools
            .into_iter()
            .map(|tool| {
                let pin = pinned.get(tool);
                let bump = bumped.get(tool);
                let entry = pin.or(bump).ok_or_else(|| tool.clone())?;
                let current = version(tool, entry, "current")?;
                Ok(Row {
                    tool: tool.clone(),
                    requested: version(tool, entry, "requested")?,
                    latest: match pin {
                        Some(pin) => version(tool, pin, "latest")?,
                        None => current.clone(),
                    },
                    current,
                    bump_latest: match bump {
                        Some(bump) => bump_latest(tool, bump)?,
                        None => None,
                    },
                })
            })
            .collect()
    }

    pub(super) fn tool(&self) -> &str {
        &self.tool
    }

    pub(super) fn requested(&self) -> Option<&str> {
        self.requested.as_deref()
    }

    pub(super) fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub(super) fn latest(&self) -> Option<&str> {
        self.latest.as_deref()
    }

    pub(super) fn bump_latest(&self) -> Option<&str> {
        self.bump_latest.as_deref()
    }
}

/// The version `entry`, the one for `tool`, gives under `key`, none where it is null, or
/// the name of the tool where the key is missing or holds something other than a string.
fn version(tool: &str, entry: &Value, key: &str) -> Result<Option<String>, String> {
    match entry.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(version)) => Ok(Some(version.clone())),
        _ => Err(tool.to_owned()),
    }
}

/// The newest version overall a `mise outdated --bump` entry reports, none where it
/// names no version to bump the request to, as the request already reaches it.
fn bump_latest(tool: &str, entry: &Value) -> Result<Option<String>, String> {
    match entry.get("bump") {
        Some(Value::Null) => Ok(None),
        Some(_) => version(tool, entry, "latest"),
        None => Err(tool.to_owned()),
    }
}
