//! The usage ccusage prints, by project.

use dotfiles_shared::Failure;
use serde_json::Value;

use super::command::Period;
use super::entry::Entry;
use super::session_logs::SessionLogs;

/// The usage ccusage reports, by project.
pub(super) struct Projects(serde_json::Map<String, Value>);

impl Projects {
    /// The projects in what ccusage printed.
    pub(super) fn parse(usage: &[u8]) -> Result<Self, Failure> {
        let usage: Value = serde_json::from_slice(usage)
            .map_err(|error| Failure::new(format_args!("ccusage printed no JSON: {error}"), 1))?;
        match usage {
            Value::Object(mut usage) => match usage.remove("projects") {
                Some(Value::Object(projects)) => Ok(Projects(projects)),
                // ccusage omits "projects" when the range holds no usage at all.
                None if usage
                    .get("totals")
                    .and_then(|totals| totals.get("totalTokens"))
                    .and_then(Value::as_u64)
                    == Some(0) =>
                {
                    Ok(Projects(serde_json::Map::new()))
                }
                _ => Err(Self::no_breakdown()),
            },
            _ => Err(Self::no_breakdown()),
        }
    }

    fn no_breakdown() -> Failure {
        Failure::new(
            format_args!("ccusage output has no per-project breakdown (\"projects\" key)"),
            1,
        )
    }

    /// Each day of each project, counted toward the repository `logs` resolve the
    /// project to, and in the row of `period` it falls in.
    pub(super) fn entries(
        &self,
        logs: &SessionLogs,
        period: Period,
    ) -> Result<Vec<Entry>, Failure> {
        let mut entries = Vec::new();
        for (project, days) in &self.0 {
            let repo = logs.repo_label(logs.cwd(project)?.as_deref())?;
            let Value::Array(days) = days else {
                return Err(Failure::new(
                    format_args!("ccusage reported no list of days for {project}: {days}"),
                    1,
                ));
            };
            for day in days {
                entries.push(Entry::from_day(day, &repo, period)?);
            }
        }
        Ok(entries)
    }
}
