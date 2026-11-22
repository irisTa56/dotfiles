//! One day of one project's usage, as ccusage reports it.

use dotfiles_shared::Failure;
use serde_json::Value;

use super::command::Period;
use super::figures::{Count, Money, Percent};

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
    /// The row the day falls in.
    pub(super) fn period(&self) -> &str {
        &self.period
    }

    /// The repository the project counts toward.
    pub(super) fn repo(&self) -> &str {
        &self.repo
    }

    /// The cost of each model the day used, by its name as ccusage gives it.
    pub(super) fn models(&self) -> &[(String, f64)] {
        &self.models
    }

    pub(super) fn usage(&self) -> Usage {
        self.usage
    }

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
pub(super) struct Usage {
    input: u64,
    output: u64,
    cache_create: u64,
    cache_read: u64,
    tokens: u64,
    cost: f64,
}

impl Usage {
    pub(super) fn cost(&self) -> f64 {
        self.cost
    }

    pub(super) fn add(self, other: Usage) -> Usage {
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
    pub(super) fn row(&self, name: &str) -> Vec<String> {
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
