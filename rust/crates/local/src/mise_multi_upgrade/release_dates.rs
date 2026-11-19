//! When the versions a run shows were released, from what `mise ls-remote` lists.

use std::collections::HashMap;

use jiff::{Timestamp, civil};
use serde_json::Value;

use super::group::Group;
use super::mise::Mise;
use super::parallel::Pool;

/// The versions each tool shown lists, by its name.
#[derive(Debug)]
pub(super) struct ReleaseDates {
    tools: HashMap<String, Versions>,
}

/// What `mise ls-remote` lists for a tool.
#[derive(Debug)]
enum Versions {
    /// Each version, with its release time, null where the backend dates none.
    Listed(HashMap<String, Value>),
    /// Nothing that could be read: mise failed, could not be started, or printed
    /// something other than a version list. It is kept apart from a version the backend
    /// lists without a time: both leave the age unknown, but only this one means the
    /// report is missing something it could have had.
    Unavailable,
}

impl ReleaseDates {
    /// Asks mise, a few tools at a time, for the versions of every tool in `groups`.
    pub(super) fn ask(groups: &[Group]) -> Self {
        let mut tools: Vec<&str> = Vec::new();
        for row in groups.iter().flat_map(Group::rows) {
            if !tools.contains(&row.tool()) {
                tools.push(row.tool());
            }
        }
        let versions = Pool::for_mise().map(&tools, |tool| Versions::ask(tool));
        ReleaseDates {
            tools: tools.into_iter().map(str::to_owned).zip(versions).collect(),
        }
    }

    /// How long ago `tool` released `version`: `-` where there is no version or the
    /// list gives it no time, and `?` where the list or the time cannot be read.
    pub(super) fn age(&self, tool: &str, version: Option<&str>) -> String {
        let Some(version) = version.filter(|version| !version.is_empty()) else {
            return "-".to_owned();
        };
        match self.tools.get(tool) {
            Some(Versions::Listed(versions)) => match versions.get(version) {
                Some(Value::String(time)) if !time.is_empty() => age(time),
                Some(Value::String(_) | Value::Null) | None => "-".to_owned(),
                Some(_) => "?".to_owned(),
            },
            Some(Versions::Unavailable) | None => "?".to_owned(),
        }
    }
}

impl Versions {
    fn ask(tool: &str) -> Self {
        let mise = Mise::new(["ls-remote", tool, "--json"].map(str::to_owned).to_vec());
        let Ok(Some(out)) = mise.output() else {
            return Versions::Unavailable;
        };
        let Ok(Value::Array(entries)) = serde_json::from_slice(&out) else {
            return Versions::Unavailable;
        };
        let mut versions = HashMap::new();
        for entry in entries {
            let Some(version) = entry.get("version") else {
                return Versions::Unavailable;
            };
            // A version that is not a string is never one a table looks up.
            if let Value::String(version) = version {
                // A backend that dates no release leaves `created_at` out.
                let time = entry.get("created_at").cloned().unwrap_or(Value::Null);
                versions.insert(version.clone(), time);
            }
        }
        Versions::Listed(versions)
    }
}

/// How long ago `time`, an ISO 8601 time with or without an offset, the latter read as
/// UTC, was: `today` under a day, whole days past that, and `?` where it cannot be read.
fn age(time: &str) -> String {
    let released = time.parse::<Timestamp>().or_else(|_| {
        time.parse::<civil::DateTime>()
            .and_then(|naive| naive.to_zoned_datetime(jiff::tz::TimeZone::UTC))
            .map(|zoned| zoned.timestamp())
    });
    let Ok(released) = released else {
        return "?".to_owned();
    };
    let days = Timestamp::now().duration_since(released).as_secs() / 86_400;
    if days < 1 {
        "today".to_owned()
    } else {
        format!("{days}d ago")
    }
}
