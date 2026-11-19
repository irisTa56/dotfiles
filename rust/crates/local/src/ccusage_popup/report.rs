//! The report: every entry of a period, shown as three tables.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::command::Period;
use super::entry::{Entry, Usage};
use super::figures::Money;
use super::table::Table;

/// Every entry of a period, shown as three tables.
pub(super) struct Report {
    period: Period,
    entries: Vec<Entry>,
}

impl Report {
    pub(super) fn new(period: Period, entries: Vec<Entry>) -> Self {
        Report { period, entries }
    }

    pub(super) fn render(&self) -> String {
        if self.entries.is_empty() {
            return "No usage data.".to_owned();
        }
        let name = self.period.name();
        let model_costs = Costs(
            self.entries
                .iter()
                .flat_map(|entry| {
                    entry.models().iter().map(|(model, cost)| {
                        (
                            entry.period().to_owned(),
                            model_name(model).to_owned(),
                            *cost,
                        )
                    })
                })
                .collect(),
        );
        let repo_costs = Costs(
            self.entries
                .iter()
                .map(|entry| {
                    (
                        entry.period().to_owned(),
                        entry.repo().to_owned(),
                        entry.usage().cost(),
                    )
                })
                .collect(),
        );
        [
            self.usage_table(&format!("Claude Code usage - {name}")),
            model_costs.table(&format!("Cost (USD) by model - {name}"), self.period),
            repo_costs.table(&format!("Cost (USD) by repository - {name}"), self.period),
        ]
        .map(|table| table.render())
        .join("\n\n")
    }

    /// Token usage and cost a period a row, with the share of input the cache served.
    fn usage_table(&self, title: &str) -> Table {
        let mut by_period: BTreeMap<&str, Usage> = BTreeMap::new();
        for entry in &self.entries {
            let usage = by_period.entry(entry.period()).or_default();
            *usage = usage.add(entry.usage());
        }
        let total = self
            .entries
            .iter()
            .fold(Usage::default(), |total, entry| total.add(entry.usage()));
        let header = [
            self.period.header(),
            "Input",
            "Output",
            "Cache Create",
            "Cache Read",
            "Total Tokens",
            "Cache Hit",
            "Cost (USD)",
        ]
        .map(str::to_owned)
        .to_vec();
        Table::new(
            title.to_owned(),
            header,
            by_period
                .iter()
                .map(|(period, usage)| usage.row(period))
                .collect(),
            total.row("Total"),
        )
    }
}

/// A model's name without the `claude-` it starts with or the date it ends with, as
/// `claude-opus-4-20250514` is shown as `opus-4`.
fn model_name(name: &str) -> &str {
    let name = name.strip_prefix("claude-").unwrap_or(name);
    let bytes = name.as_bytes();
    match bytes.len().checked_sub(9) {
        // An ASCII `-` starts a character, so the name splits there.
        Some(at) if bytes[at] == b'-' && bytes[at + 1..].iter().all(u8::is_ascii_digit) => {
            &name[..at]
        }
        _ => name,
    }
}

/// Costs as (period, key, cost) triples, a key being a model or a repository.
struct Costs(Vec<(String, String, f64)>);

impl Costs {
    /// The costs as a period-by-key table, keys ordered by their total cost with
    /// "other" last.
    fn table(&self, title: &str, period: Period) -> Table {
        let tally = self.tally();
        let keys = tally.keys();
        let periods: BTreeSet<&str> = self
            .0
            .iter()
            .map(|(period, _, _)| period.as_str())
            .collect();
        let header = std::iter::once(period.header())
            .chain(keys.iter().copied())
            .chain(std::iter::once("Total"))
            .map(str::to_owned)
            .collect();
        Table::new(
            title.to_owned(),
            header,
            periods
                .into_iter()
                .map(|period| tally.row(period, &keys))
                .collect(),
            tally.total(&keys),
        )
    }

    /// The costs summed by period and key, and by key alone.
    fn tally(&self) -> Tally<'_> {
        let mut tally = Tally {
            cells: BTreeMap::new(),
            key_totals: BTreeMap::new(),
        };
        for (period, key, cost) in &self.0 {
            *tally.cells.entry((period, key)).or_default() += cost;
            *tally.key_totals.entry(key).or_default() += cost;
        }
        tally
    }
}

/// Costs summed by (period, key), and by key, borrowing their names from [`Costs`].
struct Tally<'c> {
    cells: BTreeMap<(&'c str, &'c str), f64>,
    key_totals: BTreeMap<&'c str, f64>,
}

impl<'c> Tally<'c> {
    /// The keys by their total cost, highest first, with "other" last.
    fn keys(&self) -> Vec<&'c str> {
        let mut keys: Vec<(&str, f64)> = self
            .key_totals
            .iter()
            .map(|(key, cost)| (*key, *cost))
            .collect();
        // A stable sort, so keys of equal cost stay in the order of their names.
        keys.sort_by(|(a, a_cost), (b, b_cost)| {
            (*a == "other")
                .cmp(&(*b == "other"))
                .then(b_cost.partial_cmp(a_cost).unwrap_or(Ordering::Equal))
        });
        keys.into_iter().map(|(key, _)| key).collect()
    }

    /// The row of `period`: a cost for each of `keys`, blank where it has none, and their
    /// sum.
    fn row(&self, period: &str, keys: &[&str]) -> Vec<String> {
        let cells: Vec<Option<f64>> = keys
            .iter()
            .map(|key| self.cells.get(&(period, *key)).copied())
            .collect();
        let total: f64 = cells.iter().flatten().sum();
        std::iter::once(period.to_owned())
            .chain(
                cells
                    .iter()
                    .map(|cell| cell.map_or_else(String::new, |cost| Money::new(cost).to_string())),
            )
            .chain(std::iter::once(Money::new(total).to_string()))
            .collect()
    }

    /// The total row: each of `keys`' total cost, and the sum of them all.
    fn total(&self, keys: &[&str]) -> Vec<String> {
        std::iter::once("Total".to_owned())
            .chain(
                keys.iter()
                    .map(|key| Money::new(self.key_totals[key]).to_string()),
            )
            .chain(std::iter::once(
                Money::new(self.key_totals.values().sum()).to_string(),
            ))
            .collect()
    }
}
