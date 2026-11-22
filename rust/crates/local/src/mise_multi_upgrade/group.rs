//! A place and the tools it would upgrade.

use super::mise::Mise;
use super::place::Place;
use super::report::Report;
use super::row::{Entries, Row};
use dotfiles_shared::Failure;

/// What a place has to report.
#[derive(Debug)]
pub(super) enum Found {
    /// Tools, as a group.
    Group(Group),
    /// No tool.
    Nothing,
    /// Nothing mise could list, such as where the config is not trusted.
    Unlisted(Place),
}

/// A place with at least one tool to report, and those tools.
#[derive(Debug)]
pub(super) struct Group {
    place: Place,
    rows: Vec<Row>,
}

impl Group {
    /// What `mise outdated` reports in `place`, in the runs `report` names.
    pub(super) fn find(place: Place, report: Report) -> Result<Found, Failure> {
        let base = place.outdated_args();
        let Some(pinned) = Self::outdated(base.clone())? else {
            return Ok(Found::Unlisted(place));
        };
        let bumped = match report {
            Report::Pinned => Entries::new(),
            Report::Bump => {
                let mut args = base;
                args.push("--bump".to_owned());
                match Self::outdated(args)? {
                    Some(bumped) => bumped,
                    None => return Ok(Found::Unlisted(place)),
                }
            }
        };
        let rows = Row::merge(&pinned, &bumped).map_err(|tool| {
            Failure::new(
                format_args!(
                    "mise outdated in {} gave an entry for {tool} that lacks a version it always reports",
                    place.label()
                ),
                1,
            )
        })?;
        if rows.is_empty() {
            return Ok(Found::Nothing);
        }
        Ok(Found::Group(Group { place, rows }))
    }

    /// What `mise outdated` with `args` reports, or `None` where it fails.
    fn outdated(mut args: Vec<String>) -> Result<Option<Entries>, Failure> {
        args.push("--json".to_owned());
        let mise = Mise::new(args);
        let Some(out) = mise.output()? else {
            return Ok(None);
        };
        serde_json::from_slice(&out)
            .map(Some)
            .map_err(|error| Failure::new(format_args!("{mise} gave no JSON object: {error}"), 1))
    }

    pub(super) fn place(&self) -> &Place {
        &self.place
    }

    pub(super) fn rows(&self) -> &[Row] {
        &self.rows
    }
}
