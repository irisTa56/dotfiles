//! The report: each day's usage, and the tables it is shown as.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::Period;
use crate::failure::Failure;

mod figures;
mod table;

use figures::{Count, Money, Percent};
use table::Table;

impl Period {
    /// The word the report's titles name the period by.
    fn name(self) -> &'static str {
        match self {
            Period::Daily => "daily",
            Period::Weekly => "weekly",
            Period::Monthly => "monthly",
        }
    }

    /// The heading of the column that names each row's period.
    fn header(self) -> &'static str {
        match self {
            Period::Daily => "Date",
            Period::Weekly => "Week",
            Period::Monthly => "Month",
        }
    }
}

/// One day of one project's usage.
pub(super) struct Entry {
    /// The row the day falls in.
    period: String,
    /// The repository the project counts toward.
    repo: String,
    /// The cost of each model the day used, by its name as ccusage gives it.
    models: Vec<(String, f64)>,
    usage: Usage,
}

impl Entry {
    /// The entry for a day ccusage reports, failing on a day without a field the tables
    /// read, rather than further on, so that the failure names the day.
    pub(super) fn from_day(day: &Value, repo: &str, period: Period) -> Result<Self, Failure> {
        let date = Self::field(day, "date", Value::as_str, "text")?;
        let models = Self::field(day, "modelBreakdowns", Value::as_array, "a list")?
            .iter()
            .map(|model| {
                Ok((
                    Self::field(model, "modelName", Value::as_str, "text")?.to_owned(),
                    Self::field(model, "cost", Value::as_f64, "a number")?,
                ))
            })
            .collect::<Result<_, Failure>>()?;
        let count = |name| Self::field(day, name, Value::as_u64, "a count");
        let usage = Usage {
            input: count("inputTokens")?,
            output: count("outputTokens")?,
            cache_create: count("cacheCreationTokens")?,
            cache_read: count("cacheReadTokens")?,
            tokens: count("totalTokens")?,
            cost: Self::field(day, "totalCost", Value::as_f64, "a number")?,
        };
        let period = period.bucket(date).ok_or_else(|| {
            Failure::new(
                format_args!("ccusage reported usage whose \"date\" is no date: {day}"),
                1,
            )
        })?;
        Ok(Entry {
            period,
            repo: repo.to_owned(),
            models,
            usage,
        })
    }

    /// The field `name` of `object`, read by `read`, which fails where it is not `kind`.
    fn field<'v, T>(
        object: &'v Value,
        name: &str,
        read: impl Fn(&'v Value) -> Option<T>,
        kind: &str,
    ) -> Result<T, Failure> {
        let value = object.get(name).ok_or_else(|| {
            Failure::new(
                format_args!("ccusage reported usage without \"{name}\": {object}"),
                1,
            )
        })?;
        read(value).ok_or_else(|| {
            Failure::new(
                format_args!("ccusage reported usage whose \"{name}\" is not {kind}: {object}"),
                1,
            )
        })
    }
}

/// Token counts and their cost, of a day or summed over several.
#[derive(Debug, Clone, Copy, Default)]
struct Usage {
    input: u64,
    output: u64,
    cache_create: u64,
    cache_read: u64,
    tokens: u64,
    cost: f64,
}

impl Usage {
    fn add(self, other: Usage) -> Usage {
        Usage {
            input: self.input + other.input,
            output: self.output + other.output,
            cache_create: self.cache_create + other.cache_create,
            cache_read: self.cache_read + other.cache_read,
            tokens: self.tokens + other.tokens,
            cost: self.cost + other.cost,
        }
    }

    /// The row of the usage table that shows this usage under `name`.
    fn row(&self, name: &str) -> Vec<String> {
        // The three input counts are disjoint, so cache reads over their sum is the
        // share of all input tokens served from the cache.
        let all_input = self.input + self.cache_create + self.cache_read;
        let hit = if all_input == 0 {
            0.0
        } else {
            ratio(self.cache_read, all_input)
        };
        vec![
            name.to_owned(),
            Count::new(self.input).to_string(),
            Count::new(self.output).to_string(),
            Count::new(self.cache_create).to_string(),
            Count::new(self.cache_read).to_string(),
            Count::new(self.tokens).to_string(),
            Percent::new(hit).to_string(),
            Money::new(self.cost).to_string(),
        ]
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a token count is far below the 2^53 an f64 holds exactly"
)]
fn ratio(part: u64, whole: u64) -> f64 {
    part as f64 / whole as f64
}

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
                    entry.models.iter().map(|(model, cost)| {
                        (entry.period.clone(), model_name(model).to_owned(), *cost)
                    })
                })
                .collect(),
        );
        let repo_costs = Costs(
            self.entries
                .iter()
                .map(|entry| (entry.period.clone(), entry.repo.clone(), entry.usage.cost))
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
            let usage = by_period.entry(&entry.period).or_default();
            *usage = usage.add(entry.usage);
        }
        let total = self
            .entries
            .iter()
            .fold(Usage::default(), |total, entry| total.add(entry.usage));
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
