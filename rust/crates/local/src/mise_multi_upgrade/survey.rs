//! What every place has to report.

use std::io::{self, Write};

use super::group::{Found, Group};
use super::parallel::Pool;
use super::place::Place;
use super::question::{Answer, Question};
use super::release_dates::ReleaseDates;
use super::report::Report;
use super::table::Table;
use crate::failure::Failure;

/// The places with tools to report, and those mise could not list, each in the order
/// the places were given.
#[derive(Debug)]
pub(super) struct Survey {
    groups: Vec<Group>,
    skipped: Vec<Place>,
}

impl Survey {
    /// Asks every place in `places`, a few at a time, for the runs `report` names.
    pub(super) fn take(places: &[Place], report: Report) -> Result<Self, Failure> {
        let found = Pool::for_mise().map(places, |place| Group::find(place.clone(), report));
        let mut survey = Survey {
            groups: Vec::new(),
            skipped: Vec::new(),
        };
        for found in found {
            match found? {
                Found::Group(group) => survey.groups.push(group),
                Found::Nothing => {}
                Found::Unlisted(place) => survey.skipped.push(place),
            }
        }
        Ok(survey)
    }

    pub(super) fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// Shows every group's table, with the columns `report` names.
    ///
    /// A reader that stops early, as `| head` does or quitting a pager, closes the pipe,
    /// which ends the report rather than failing it.
    pub(super) fn show(&self, dates: &ReleaseDates, report: Report) -> Result<(), Failure> {
        let mut stdout = io::stdout();
        for group in &self.groups {
            match write!(stdout, "{}", Table::new(group, dates, report).render()) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
                Err(error) => return Err(Failure::writing(&error)),
            }
        }
        Ok(())
    }

    /// Shows each group's table and asks whether to upgrade its place, every question
    /// coming before any upgrade, so the run needs nobody once the last is answered.
    /// The places answered yes, in order.
    ///
    /// A closed stdout fails the run here, as an upgrade nobody saw asked for must not
    /// run.
    pub(super) fn ask(&self, dates: &ReleaseDates) -> Result<Vec<Place>, Failure> {
        let mut accepted = Vec::new();
        let mut stdout = io::stdout();
        for group in &self.groups {
            write!(
                stdout,
                "{}",
                Table::new(group, dates, Report::Pinned).render()
            )
            .map_err(|error| Failure::writing(&error))?;
            let question = Question::new(format!("Upgrade {}? [y/N] ", group.place().label()));
            match question.ask()? {
                Answer::Yes => accepted.push(group.place().clone()),
                Answer::No => {}
                Answer::End => {
                    writeln!(stdout).map_err(|error| Failure::writing(&error))?;
                    break;
                }
            }
        }
        Ok(accepted)
    }

    /// Names the places mise could not list, last, where they are read; mise's own
    /// errors have already gone to stderr.
    pub(super) fn report_skipped(&self) {
        if self.skipped.is_empty() {
            return;
        }
        let labels: Vec<String> = self.skipped.iter().map(Place::label).collect();
        let _ = writeln!(
            io::stderr(),
            "\nSkipped, as mise could not list them: {}",
            labels.join(", ")
        );
    }
}
