//! `ccusage-popup`: Claude Code usage over a period, as tables in a Quick Look popup.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::civil::Date;
use jiff::{ToSpan, ZonedDateTime};
use serde_json::Value;

use crate::failure::{Failure, exit_code};

const USAGE: &str = "\
Usage: mise run ccusage:<daily|weekly|monthly>

Shows Claude Code usage in a Quick Look popup as three tables per period:
token usage with the cache hit rate, cost by model, and cost by repository.
Periods cover the last 2 weeks, the last 9 weeks starting on a Sunday (as
`ccusage claude weekly` does by default), or the last 12 months.
";

/// Shows the usage over the period `args` names.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let Some(period) = Period::from_args(args) else {
        // The usage ends in a blank line, as it did where it was printed with a line end
        // of its own after it.
        return Failure::new(format_args!("{USAGE}"), 2).report();
    };
    match period.show() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

/// The span the usage is shown over, and the unit its rows are.
#[derive(Debug, Clone, Copy)]
enum Period {
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

    /// The row a day falls in, from the date ccusage gives it, as `YYYY-MM-DD`.
    fn bucket(self, date: &str) -> Option<String> {
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

/// The usage ccusage reports, by project.
struct Projects(serde_json::Map<String, Value>);

impl Projects {
    /// The projects in what ccusage printed.
    fn parse(usage: &[u8]) -> Result<Self, Failure> {
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
    fn entries(&self, logs: &SessionLogs, period: Period) -> Result<Vec<Entry>, Failure> {
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

/// One day of one project's usage.
struct Entry {
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
    fn from_day(day: &Value, repo: &str, period: Period) -> Result<Self, Failure> {
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
            Count(self.input).to_string(),
            Count(self.output).to_string(),
            Count(self.cache_create).to_string(),
            Count(self.cache_read).to_string(),
            Count(self.tokens).to_string(),
            Percent(hit).to_string(),
            Money(self.cost).to_string(),
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
struct Report {
    period: Period,
    entries: Vec<Entry>,
}

impl Report {
    fn new(period: Period, entries: Vec<Entry>) -> Self {
        Report { period, entries }
    }

    fn render(&self) -> String {
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
        Table {
            title: title.to_owned(),
            header,
            body: by_period
                .iter()
                .map(|(period, usage)| usage.row(period))
                .collect(),
            total: total.row("Total"),
        }
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
        let mut cells: BTreeMap<(&str, &str), f64> = BTreeMap::new();
        let mut key_totals: BTreeMap<&str, f64> = BTreeMap::new();
        for (period, key, cost) in &self.0 {
            *cells.entry((period, key)).or_default() += cost;
            *key_totals.entry(key).or_default() += cost;
        }
        let mut keys: Vec<(&str, f64)> =
            key_totals.iter().map(|(key, cost)| (*key, *cost)).collect();
        // A stable sort, so keys of equal cost stay in the order of their names.
        keys.sort_by(|(a, a_cost), (b, b_cost)| {
            (*a == "other").cmp(&(*b == "other")).then(
                b_cost
                    .partial_cmp(a_cost)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        let periods: std::collections::BTreeSet<&str> = self
            .0
            .iter()
            .map(|(period, _, _)| period.as_str())
            .collect();
        let body =
            periods
                .into_iter()
                .map(|period| {
                    let row: Vec<Option<f64>> = keys
                        .iter()
                        .map(|(key, _)| cells.get(&(period, *key)).copied())
                        .collect();
                    let total: f64 = row.iter().flatten().sum();
                    std::iter::once(period.to_owned())
                        .chain(row.iter().map(|cell| {
                            cell.map_or_else(String::new, |cost| Money(cost).to_string())
                        }))
                        .chain(std::iter::once(Money(total).to_string()))
                        .collect()
                })
                .collect();
        let header = std::iter::once(period.header().to_owned())
            .chain(keys.iter().map(|(key, _)| (*key).to_owned()))
            .chain(std::iter::once("Total".to_owned()))
            .collect();
        let total = std::iter::once("Total".to_owned())
            .chain(keys.iter().map(|(_, cost)| Money(*cost).to_string()))
            .chain(std::iter::once(
                Money(key_totals.values().sum()).to_string(),
            ))
            .collect();
        Table {
            title: title.to_owned(),
            header,
            body,
            total,
        }
    }
}

/// A titled table of a header row, body rows and a total row, each of cell texts.
struct Table {
    title: String,
    header: Vec<String>,
    body: Vec<Vec<String>>,
    total: Vec<String>,
}

impl Table {
    /// The table as text, the first column left-aligned and the rest, all numbers,
    /// right-aligned, with a rule under the header and above the total.
    fn render(&self) -> String {
        let rows = || {
            std::iter::once(&self.header)
                .chain(&self.body)
                .chain(std::iter::once(&self.total))
        };
        let widths: Vec<usize> = (0..self.header.len())
            .map(|column| {
                rows()
                    .map(|row| row[column].chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let rule = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  ");
        let line = |row: &[String]| {
            row.iter()
                .zip(&widths)
                .enumerate()
                .map(|(column, (cell, width))| {
                    if column == 0 {
                        format!("{cell:<width$}")
                    } else {
                        format!("{cell:>width$}")
                    }
                })
                .collect::<Vec<_>>()
                .join("  ")
        };
        let mut lines = vec![
            self.title.clone(),
            String::new(),
            line(&self.header),
            rule.clone(),
        ];
        lines.extend(self.body.iter().map(|row| line(row)));
        lines.push(rule);
        lines.push(line(&self.total));
        lines.join("\n")
    }
}

/// A count of tokens, shown with a comma between each group of three digits.
struct Count(u64);

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&group_thousands(&self.0.to_string()))
    }
}

/// `digits` with a comma before each group of three from the right, after a sign
/// where there is one.
fn group_thousands(digits: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |digits| ("-", digits));
    let mut grouped = String::from(sign);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// A cost in dollars, shown to the cent.
///
/// ccusage reports each cost as a float in dollars. Every figure shown, a cell or a
/// total, is its own sum rounded once here, so each is the nearest cent to what it
/// stands for, and the cells of a row or column need not add up to its total: over many
/// small costs they can be several cents apart. Rounding each cost to cents before
/// summing would make them add up by moving every figure off its own value instead.
struct Money(f64);

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a cost in cents is far inside an i64"
        )]
        let cents = (self.0 * 100.0).round() as i64;
        let dollars = group_thousands(&(cents / 100).to_string());
        write!(f, "${dollars}.{:02}", cents % 100)
    }
}

/// A share, shown as a percentage to one decimal place.
struct Percent(f64);

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a share in tenths of a percent is at most 1000"
        )]
        let permille = (self.0 * 1000.0).round() as i64;
        write!(f, "{}.{}%", permille / 10, permille % 10)
    }
}

/// The session logs ccusage reads, through which each project is resolved to the
/// repository it counts toward.
///
/// ccusage names each project after the session's cwd with every non-alphanumeric
/// character replaced by "-". That mapping is lossy ("my-repo" and "my/repo" collide),
/// so each project is resolved through the real cwd recorded in its session logs
/// instead of by parsing the name:
///
/// - a worktree under `<repo>/.claude/worktrees/` counts toward `<repo>`;
/// - a session started in a Claude scratchpad (`/private/tmp/claude-<uid>/<parent
///   project>/<session>/scratchpad/...`) counts toward the repository of the parent
///   project that launched it;
/// - anything that is not a git repository counts toward "other".
struct SessionLogs {
    /// The projects directories ccusage reads.
    dirs: Vec<PathBuf>,
}

impl SessionLogs {
    /// The projects directories ccusage reads. Each comma-separated `CLAUDE_CONFIG_DIR`
    /// entry counts when it holds `projects/` or is itself named `projects`. Without the
    /// variable, ccusage reads `projects/` under `$XDG_CONFIG_HOME/claude`
    /// (`~/.config/claude` when that is unset) and under `~/.claude`.
    fn from_env() -> Result<Self, Failure> {
        let dirs = if let Some(entries) = std::env::var_os("CLAUDE_CONFIG_DIR") {
            entries
                .to_string_lossy()
                .split(',')
                .map(str::trim)
                .filter(|dir| !dir.is_empty())
                .filter_map(|dir| {
                    let projects = Path::new(dir).join("projects");
                    if projects.is_dir() {
                        Some(projects)
                    } else if Path::new(dir).file_name() == Some(OsStr::new("projects")) {
                        Some(PathBuf::from(dir))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            let home = std::env::home_dir()
                .ok_or_else(|| Failure::new(format_args!("cannot find the home directory"), 1))?;
            let xdg = std::env::var_os("XDG_CONFIG_HOME")
                .map_or_else(|| home.join(".config"), PathBuf::from);
            vec![xdg.join("claude/projects"), home.join(".claude/projects")]
        };
        Ok(SessionLogs { dirs })
    }

    /// The cwd recorded in the first session log of `project` that records one.
    fn cwd(&self, project: &str) -> Result<Option<String>, Failure> {
        for dir in &self.dirs {
            let mut logs: Vec<PathBuf> = match fs::read_dir(dir.join(project)) {
                Ok(entries) => entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    // What `*.jsonl` matches: no hidden file, and the case as it is.
                    .filter(|path| {
                        path.extension()
                            .is_some_and(|extension| extension == "jsonl")
                            && path
                                .file_name()
                                .is_some_and(|name| !name.as_encoded_bytes().starts_with(b"."))
                    })
                    .collect(),
                Err(_) => continue,
            };
            logs.sort();
            for log in logs {
                if let Some(cwd) = Self::log_cwd(&log)? {
                    return Ok(Some(cwd));
                }
            }
        }
        Ok(None)
    }

    /// The cwd the first line of `log` that records one records.
    fn log_cwd(log: &Path) -> Result<Option<String>, Failure> {
        let unreadable = |error: io::Error| {
            Failure::new(format_args!("cannot read {}: {error}", log.display()), 1)
        };
        let mut reader = BufReader::new(fs::File::open(log).map_err(unreadable)?);
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line).map_err(unreadable)? == 0 {
                return Ok(None);
            }
            // A log's other lines, such as its summary, record no cwd.
            if let Ok(Value::Object(record)) = serde_json::from_slice(&line)
                && let Some(Value::String(cwd)) = record.get("cwd")
            {
                return Ok(Some(cwd.clone()));
            }
        }
    }

    /// The repository label a session's cwd counts toward, "other" where none is known.
    fn repo_label(&self, cwd: Option<&str>) -> Result<String, Failure> {
        let Some(cwd) = cwd else {
            return Ok("other".to_owned());
        };
        if let Some(parent) = scratchpad_parent(cwd) {
            // A parent's name is a strict part of this path, so recursion ends.
            return self.repo_label(self.cwd(parent)?.as_deref());
        }
        let checkout = cwd
            .split_once("/.claude/worktrees/")
            .map_or(cwd, |(checkout, _)| checkout);
        checkout_label(checkout)
    }
}

/// The parent project of a session started in a Claude scratchpad, which matches
/// `^(?:/private)?/tmp/claude-\d+/([^/]+)/`.
fn scratchpad_parent(cwd: &str) -> Option<&str> {
    let rest = cwd.strip_prefix("/private").unwrap_or(cwd);
    let rest = rest.strip_prefix("/tmp/claude-")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let (parent, _) = rest[digits..].strip_prefix('/')?.split_once('/')?;
    (!parent.is_empty()).then_some(parent)
}

/// The label of the checkout at `dir`: its repository's name, "other" where it is no
/// repository.
fn checkout_label(dir: &str) -> Result<String, Failure> {
    let name = |path: &Path| {
        path.file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_owned()
    };
    if !Path::new(dir).is_dir() {
        // A deleted temporary directory is not a repository; any other missing
        // directory is most likely a repository that has since been moved.
        return Ok(
            if ["/tmp/", "/private/", "/var/"]
                .iter()
                .any(|prefix| dir.starts_with(prefix))
            {
                "other".to_owned()
            } else {
                name(Path::new(dir))
            },
        );
    }
    let output = Command::new("git")
        .args([
            "-C",
            dir,
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| Failure::new(format_args!("cannot run git: {error}"), 1))?;
    if !output.status.success() {
        return Ok("other".to_owned());
    }
    // The common dir is shared by a checkout, its subdirectories and its linked
    // worktrees: <repo>/.git normally, <repo>.git for a bare repository, and
    // .git/modules/<repo> for a submodule.
    let common = String::from_utf8_lossy(&output.stdout);
    let common = Path::new(common.trim());
    let common_name = name(common);
    Ok(if common_name == ".git" {
        common.parent().map_or_else(String::new, name)
    } else {
        common_name
            .strip_suffix(".git")
            .unwrap_or(&common_name)
            .to_owned()
    })
}

/// A report written as HTML in a directory of its own under the temporary one, both
/// removed when this is dropped.
struct ReportFile {
    dir: PathBuf,
}

impl ReportFile {
    fn create(report: &str) -> Result<Self, Failure> {
        Self::write(report)
            .map_err(|error| Failure::new(format_args!("cannot write the report: {error}"), 1))
    }

    fn write(report: &str) -> io::Result<Self> {
        let dir = Self::new_dir()?;
        let report_file = ReportFile { dir };
        let body = report
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        fs::write(
            report_file.html(),
            format!(
                "<meta charset=\"utf-8\"><body style=\"margin:0;background:#1e1e1e\">\n\
                 <pre style=\"font:13px ui-monospace,Menlo,monospace;color:#eee;padding:16px\">\n\
                 {body}\n\
                 </pre>\n"
            ),
        )?;
        Ok(report_file)
    }

    /// A directory no other run has, since `create_dir` fails on a name already taken
    /// rather than sharing it.
    fn new_dir() -> io::Result<PathBuf> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let base = format!("ccusage.{}.{nanos}", std::process::id());
        let mut attempt = 0;
        loop {
            let dir = std::env::temp_dir().join(format!("{base}.{attempt}"));
            match fs::create_dir(&dir) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => attempt += 1,
                result => return result.map(|()| dir),
            }
        }
    }

    /// Shows the report in a Quick Look panel, until it is dismissed.
    fn show(&self) -> Result<(), Failure> {
        // Quick Look renders HTML via WebKit, guaranteeing a monospace font and exact
        // column alignment that a proportional AppleScript dialog would mangle.
        // qlmanage -p blocks until the preview panel is dismissed. It is noisy on stderr
        // even on success, so its output is dropped, but an outright render failure still
        // surfaces instead of doing nothing.
        let status = Command::new("qlmanage")
            .arg("-p")
            .arg(self.html())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|error| Failure::new(format_args!("cannot run qlmanage: {error}"), 1))?;
        if status.success() {
            Ok(())
        } else {
            Err(Failure::new(format_args!("Quick Look preview failed"), 1))
        }
    }

    fn html(&self) -> PathBuf {
        self.dir.join("report.html")
    }
}

impl Drop for ReportFile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
