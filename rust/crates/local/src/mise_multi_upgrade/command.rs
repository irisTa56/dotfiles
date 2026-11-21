//! The command: its usage, and the stages a run goes through.

use std::io::{self, Write};

use clap::Args;

use super::options::Options;
use super::place::Place;
use super::release_dates::ReleaseDates;
use super::survey::Survey;
use super::tracked::Tracked;
use super::upgrades::Upgrades;
use crate::failure::Failure;

/// Upgrade mise tools directory by directory, asking for each first.
///
/// Shows, for each directory with a tracked mise config, the tools `mise upgrade` there
/// would upgrade, with the age of each version, and asks whether to upgrade it. Once
/// every directory is answered, runs `mise upgrade` in the accepted ones, keeping each
/// version request as it is. The global config counts as a directory of its own,
/// upgraded with `mise upgrade --global`; a project is upgraded with `--no-global`, so a
/// global tool is upgraded once.
///
/// Latest is the newest version matching the request, which the upgrade installs; Bump
/// Latest, shown with --bump, is the newest version overall, reachable only by changing
/// the request, and is shown for information. A `-` marks nothing to report; a `?` marks
/// a Released cell whose age could not be determined.
///
/// Run it as `mise run mise:multi-upgrade`, and for this help, `mise run
/// mise:multi-upgrade -- --help`, since mise answers a bare `--help` itself.
#[derive(Debug, Args)]
pub(crate) struct Request {
    #[command(flatten)]
    options: Options,
}

impl Request {
    /// Upgrades, or with `--dry-run` reports, what the options ask for.
    pub(crate) fn run(self) -> Result<(), Failure> {
        upgrade(&self.options)
    }
}

/// Reports what every place would upgrade, and unless it is a dry run, asks and
/// upgrades.
fn upgrade(options: &Options) -> Result<(), Failure> {
    let report = options.report();
    let survey = Survey::take(&Tracked::list(options)?.places(), report)?;
    let dates = ReleaseDates::ask(survey.groups());
    let failed = if survey.groups().is_empty() {
        let message = if options.dry_run() {
            "All tools are up to date."
        } else {
            "Nothing to upgrade within the requested versions."
        };
        writeln!(io::stdout(), "{message}").map_err(|error| Failure::writing(&error))?;
        Vec::new()
    } else if options.dry_run() {
        survey.show(&dates, report)?;
        Vec::new()
    } else {
        Upgrades::new(survey.ask(&dates)?).run()?
    };
    survey.report_skipped();
    if failed.is_empty() {
        return Ok(());
    }
    let labels: Vec<String> = failed.iter().map(Place::label).collect();
    Err(Failure::new(
        format_args!("Upgrade failed in: {}", labels.join(", ")),
        1,
    ))
}
