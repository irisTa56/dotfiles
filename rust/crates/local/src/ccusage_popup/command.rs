//! The command: the period it is run for, and the stages it runs through.

use std::process::{Command, Stdio};

use jiff::civil::Date;
use jiff::{ToSpan, ZonedDateTime};

use super::projects::Projects;
use super::report::Report;
use super::report_file::ReportFile;
use super::session_logs::SessionLogs;
use crate::failure::{Failure, exit_code};

const USAGE: &str = "\
Usage: mise run ccusage:<daily|weekly|monthly>

Shows Claude Code usage in a Quick Look popup as three tables per period:
token usage with the cache hit rate, cost by model, and cost by repository.
Periods cover the last 2 weeks, the last 9 weeks starting on a Sunday (as
`ccusage claude weekly` does by default), or the last 12 months.
";

/// Shows the usage over the period `args` names.
pub(crate) fn run(args: &[String]) -> Result<(), Failure> {
    let Some(period) = Period::from_args(args) else {
        // The usage's own last line end and the one the report adds leave a blank line
        // after it.
        return Err(Failure::new(format_args!("{USAGE}"), 2));
    };
    period.show()
}

/// The span the usage is shown over, and the unit its rows are.
#[derive(Debug, Clone, Copy)]
pub(super) enum Period {
    /// The last 2 weeks, a row a day.
    Daily,
    /// The last 9 weeks, a row a week from Sunday, as `ccusage claude weekly` counts them.
    Weekly,
    /// The last 12 months, a row a month.
    Monthly,
}

impl Period {
    fn from_args(args: &[String]) -> Option<Self> {
        match args {
            [period] if period == "daily" => Some(Period::Daily),
            [period] if period == "weekly" => Some(Period::Weekly),
            [period] if period == "monthly" => Some(Period::Monthly),
            _ => None,
        }
    }

    /// The first day the period covers, counted back from `today`.
    fn since(self, today: Date) -> Date {
        match self {
            Period::Daily => today - 14.days(),
            Period::Weekly => week_start(today) - 56.days(),
            Period::Monthly => today.first_of_month() - 11.months(),
        }
    }

    fn show(self) -> Result<(), Failure> {
        let projects = Projects::parse(&self.usage()?)?;
        let entries = projects.entries(&SessionLogs::from_env()?, self)?;
        ReportFile::create(&Report::new(self, entries).render())?.show()
    }

    /// What ccusage prints of each project's usage over the period, a day at a time, its
    /// errors left on stderr.
    fn usage(self) -> Result<Vec<u8>, Failure> {
        // The local date, which is also the date ccusage groups usage by.
        let today = ZonedDateTime::now().date();
        let since = self.since(today).strftime("%Y%m%d").to_string();
        let args = [
            "claude",
            "daily",
            "--instances",
            "--json",
            "--since",
            &since,
        ];
        let output = Command::new("ccusage")
            .args(args)
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| Failure::new(format_args!("cannot run ccusage: {error}"), 1))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let code = exit_code(output.status);
        Err(Failure::new(
            format_args!("ccusage {} failed (exit {code})", args.join(" ")),
            code,
        ))
    }

    /// The word the report's titles name the period by.
    pub(super) fn name(self) -> &'static str {
        match self {
            Period::Daily => "daily",
            Period::Weekly => "weekly",
            Period::Monthly => "monthly",
        }
    }

    /// The heading of the column that names each row's period.
    pub(super) fn header(self) -> &'static str {
        match self {
            Period::Daily => "Date",
            Period::Weekly => "Week",
            Period::Monthly => "Month",
        }
    }

    /// The row a day falls in, from the date ccusage gives it, as `YYYY-MM-DD`.
    pub(super) fn bucket(self, date: &str) -> Option<String> {
        match self {
            Period::Daily => Some(date.to_owned()),
            Period::Weekly => {
                let date: Date = date.parse().ok()?;
                Some(week_start(date).to_string())
            }
            Period::Monthly => Some(date.chars().take(7).collect()),
        }
    }
}

/// The Sunday that starts the week `date` falls in.
fn week_start(date: Date) -> Date {
    date - i64::from(date.weekday().to_sunday_zero_offset()).days()
}
