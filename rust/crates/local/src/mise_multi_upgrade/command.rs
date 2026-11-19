//! The command: its usage, and the stages a run goes through.

use std::io::{self, Write};
use std::process::ExitCode;

use super::options::Options;
use super::place::Place;
use super::release_dates::ReleaseDates;
use super::survey::Survey;
use super::tracked::Tracked;
use super::upgrades::Upgrades;
use crate::failure::Failure;

const USAGE: &str = "\
Usage: mise run mise:multi-upgrade [--dry-run [--bump]] [--include-worktrees] [--include-tmpdirs]

Shows, for each directory with a tracked mise config, the tools `mise upgrade`
there would upgrade, with the age of each version, and asks whether to upgrade
it. Once every directory is answered, runs `mise upgrade` in the accepted ones,
keeping each version request as it is. The global config counts as a directory
of its own, upgraded with `mise upgrade --global`; a project is upgraded with
`--no-global`, so a global tool is upgraded once.

  --dry-run, -n        show every directory's table, and ask and upgrade nothing
  --bump               with --dry-run, also show for each tool the newest version
                       overall, and the tools newer only past their request
  --include-worktrees  also process linked git worktrees, such as agents' worktrees
  --include-tmpdirs    also process configs under a temporary directory
  --help, -h           print this message; through the task, `-- --help`, since
                       mise answers a bare `--help` itself

Latest is the newest version matching the request, which the upgrade installs;
Bump Latest, shown with --bump, is the newest version overall, reachable only
by changing the request, and is shown for information. A `-` marks nothing to
report; a `?` marks a Released cell whose age could not be determined.
";

/// Upgrades, or with `--dry-run` reports, what the options `args` give ask for.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let Some(options) = Options::parse(args) else {
        return Failure::new(format_args!("{USAGE}"), 2).report();
    };
    if options.help() {
        let _ = writeln!(io::stdout(), "{USAGE}");
        return ExitCode::SUCCESS;
    }
    // The upgrade stays within each request, so a bump is only ever a report, and one
    // shown beside the question would read as an offer.
    if options.bump() && !options.dry_run() {
        return Failure::new(format_args!("--bump needs --dry-run\n\n{USAGE}"), 2).report();
    }
    match upgrade(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
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
