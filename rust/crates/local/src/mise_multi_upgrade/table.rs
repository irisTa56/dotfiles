//! A group's tools laid out as a table.

use super::group::Group;
use super::release_dates::ReleaseDates;
use super::report::Report;
use super::row::Row;

const HEADERS: [&str; 6] = [
    "Tool",
    "Requested",
    "Current",
    "Released",
    "Latest",
    "Released",
];
const BUMP_HEADERS: [&str; 2] = ["Bump Latest", "Released"];

/// A group shown with the columns a report has, each version with its age.
pub(super) struct Table<'a> {
    group: &'a Group,
    dates: &'a ReleaseDates,
    report: Report,
}

impl<'a> Table<'a> {
    pub(super) fn new(group: &'a Group, dates: &'a ReleaseDates, report: Report) -> Self {
        Table {
            group,
            dates,
            report,
        }
    }

    /// The table, after a blank line and a heading naming the group's place, each
    /// column as wide as its widest cell.
    pub(super) fn render(&self) -> String {
        let headers = self.headers();
        let cells: Vec<Vec<String>> = self
            .group
            .rows()
            .iter()
            .map(|row| self.cells(row))
            .collect();
        let widths = widths(&headers, &cells);
        let rule: Vec<String> = widths.iter().map(|&width| "-".repeat(width)).collect();
        let mut text = format!("\n{}\n", self.group.place().label());
        for line in std::iter::once(&headers).chain([&rule]).chain(&cells) {
            text.push_str(&layout(line, &widths));
            text.push('\n');
        }
        text
    }

    fn headers(&self) -> Vec<String> {
        let mut headers: Vec<String> = HEADERS.map(str::to_owned).to_vec();
        if self.report == Report::Bump {
            headers.extend(BUMP_HEADERS.map(str::to_owned));
        }
        headers
    }

    fn cells(&self, row: &Row) -> Vec<String> {
        let mut cells = vec![
            row.tool().to_owned(),
            version(row.requested()),
            version(row.current()),
            self.dates.age(row.tool(), row.current()),
            version(row.latest()),
            self.dates.age(row.tool(), row.latest()),
        ];
        if self.report == Report::Bump {
            cells.push(version(row.bump_latest()));
            cells.push(self.dates.age(row.tool(), row.bump_latest()));
        }
        cells
    }
}

/// How wide each column is: as wide as its widest cell, the header's included.
fn widths(headers: &[String], cells: &[Vec<String>]) -> Vec<usize> {
    (0..headers.len())
        .map(|column| {
            std::iter::once(headers)
                .chain(cells.iter().map(Vec::as_slice))
                .map(|line| line[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect()
}

/// A line of `cells`, each padded to its column's width, two spaces apart, with no
/// trailing space.
fn layout(cells: &[String], widths: &[usize]) -> String {
    let padded: Vec<String> = cells
        .iter()
        .zip(widths)
        .map(|(cell, &width)| format!("{cell:<width$}"))
        .collect();
    padded.join("  ").trim_end().to_owned()
}

/// A version as a cell shows it, `-` where there is none.
fn version(version: Option<&str>) -> String {
    version.unwrap_or("-").to_owned()
}
