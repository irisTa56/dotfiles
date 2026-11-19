//! `ccusage-popup`: Claude Code usage as tables in a Quick Look popup, run with fakes for
//! ccusage, git and qlmanage, the last standing in for macOS's since the tests run on
//! Linux too.
//!
//! The sandbox lies under the system's temporary directory rather than in this
//! repository's `target/`: in an agent's worktree that path holds `.claude/worktrees/`,
//! which the command counts toward the checkout above it.

use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use jiff::civil::{Date, Weekday};
use jiff::{ToSpan, ZonedDateTime};

/// A directory of the test's own, holding session logs, the fakes and the temporary
/// directory the command writes its report under, removed when the test ends.
struct Sandbox {
    root: PathBuf,
}

/// What a run left: its exit status, its stderr, and the HTML Quick Look was handed,
/// if the run got that far.
struct Run {
    code: Option<i32>,
    err: String,
    html: Option<String>,
}

impl Sandbox {
    /// A fresh sandbox named after `test`, whose ccusage answers `usage` with status 0,
    /// whose git knows no repository, and whose qlmanage succeeds.
    fn new(test: &str, usage: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "dotfiles-local-ccusage-popup-{}-{test}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in ["bin", "tmp", "config/projects", "home"] {
            fs::create_dir_all(root.join(dir)).expect("the sandbox is created");
        }
        let sandbox = Sandbox { root };
        sandbox.ccusage(usage, 0);
        sandbox.repositories(&[]);
        sandbox.quick_look(0);
        sandbox
    }

    fn fake(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
    }

    /// Makes ccusage print `usage` and exit with `code`, recording its arguments one call
    /// a line.
    fn ccusage(&self, usage: &str, code: u8) {
        let answer = self.root.join("usage.json");
        fs::write(&answer, usage).expect("the usage is written");
        self.fake(
            "ccusage",
            &format!(
                "echo \"$*\" >> '{log}'\n/bin/cat '{answer}'\necho 'ccusage warns' >&2\nexit {code}\n",
                log = self.root.join("ccusage.log").display(),
                answer = answer.display(),
            ),
        );
    }

    /// Makes git report, for each directory, the common dir given beside it, and fail
    /// as outside a repository for any other, recording the calls it does not expect.
    fn repositories(&self, commons: &[(&Path, &str)]) {
        let cases = commons
            .iter()
            .fold(String::new(), |mut cases, (dir, common)| {
                let _ = writeln!(cases, "  '{}') echo '{common}'; exit 0 ;;", dir.display());
                cases
            });
        self.fake(
            "git",
            &format!(
                "if [ \"$1 $3 $4 $5\" != '-C rev-parse --path-format=absolute --git-common-dir' ]; then\n\
                 echo \"$*\" >> '{log}'; exit 2\nfi\n\
                 case \"$2\" in\n{cases}esac\n\
                 echo 'fatal: not a git repository' >&2\nexit 128\n",
                log = self.root.join("git.unexpected").display(),
            ),
        );
    }

    /// Makes qlmanage copy the file it is handed beside its path, and exit with `code`.
    fn quick_look(&self, code: u8) {
        self.fake(
            "qlmanage",
            &format!(
                "[ \"$1\" = -p ] || exit 2\necho \"$2\" > '{path}'\n/bin/cat \"$2\" > '{html}'\n\
                 echo 'qlmanage chatters' >&2\nexit {code}\n",
                path = self.root.join("html.path").display(),
                html = self.root.join("html").display(),
            ),
        );
    }

    /// The directory a project's session logs live in, under the config dir the runs name.
    fn config(&self) -> PathBuf {
        self.root.join("config")
    }

    /// Runs `ccusage-popup` with `args`, the fakes first on PATH, `CLAUDE_CONFIG_DIR`
    /// naming the sandbox's config dir and `TMPDIR` the sandbox's own.
    fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[("CLAUDE_CONFIG_DIR", self.config().as_os_str())])
    }

    fn run_with(&self, args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Run {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let path = std::env::join_paths(
            std::iter::once(self.root.join("bin")).chain(std::env::split_paths(&inherited)),
        )
        .expect("PATH is joined");
        let output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("ccusage-popup")
            .args(args)
            .env("PATH", path)
            .env("TMPDIR", self.root.join("tmp"))
            .env("HOME", self.root.join("home"))
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("XDG_CONFIG_HOME")
            .envs(env.iter().copied())
            .output()
            .expect("dotfiles-local starts");
        let html = fs::read_to_string(self.root.join("html")).ok();
        let _ = fs::remove_file(self.root.join("html"));
        Run {
            code: output.status.code(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
            html,
        }
    }

    /// Asserts that the report Quick Look was handed, and the directory made for it, are
    /// gone, and that nothing else is left in the temporary directory.
    fn assert_report_removed(&self) {
        let left: Vec<_> = fs::read_dir(self.root.join("tmp"))
            .expect("the temporary directory is read")
            .map(|entry| entry.expect("an entry is read").path())
            .collect();
        assert_eq!(left, Vec::<PathBuf>::new());
    }

    /// The arguments of each ccusage call, joined by spaces.
    fn ccusage_calls(&self) -> Vec<String> {
        self.log("ccusage.log")
    }

    fn log(&self, name: &str) -> Vec<String> {
        fs::read_to_string(self.root.join(name)).map_or_else(
            |_| Vec::new(),
            |log| log.lines().map(str::to_owned).collect(),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// What ccusage prints for a range that holds no usage at all.
const NO_USAGE: &str = r#"{"totals": {"totalTokens": 0}}"#;

#[test]
fn refuses_another_period_with_its_usage_and_exit_2() {
    let sandbox = Sandbox::new("another-period", NO_USAGE);

    let run = sandbox.run(&["yearly"]);

    assert_eq!(run.code, Some(2), "{}", run.err);
    assert!(
        run.err
            .starts_with("Usage: mise run ccusage:<daily|weekly|monthly>\n"),
        "{}",
        run.err
    );
    assert_eq!(sandbox.ccusage_calls(), Vec::<String>::new());
    assert_eq!(run.html, None);
}

/// The date ccusage groups usage by: the local one.
fn today() -> Date {
    ZonedDateTime::now().date()
}

#[test]
fn asks_ccusage_for_the_last_two_weeks_of_daily_usage_per_project() {
    let sandbox = Sandbox::new("daily-range", NO_USAGE);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    let since = (today() - 14.days()).strftime("%Y%m%d").to_string();
    assert_eq!(
        sandbox.ccusage_calls(),
        [format!("claude daily --instances --json --since {since}")]
    );
}

#[test]
fn asks_for_nine_weeks_from_a_sunday_and_twelve_months_from_the_first() {
    let sandbox = Sandbox::new("weekly-monthly-range", NO_USAGE);

    sandbox.run(&["weekly"]);
    sandbox.run(&["monthly"]);

    let since: Vec<Date> = sandbox
        .ccusage_calls()
        .iter()
        .map(|call| {
            let since = call.rsplit(' ').next().expect("the call names a date");
            Date::strptime("%Y%m%d", since).expect("the date is YYYYMMDD")
        })
        .collect();
    let [weekly, monthly] = since[..] else {
        panic!("two calls: {since:?}");
    };
    assert_eq!(weekly.weekday(), Weekday::Sunday);
    let days = (today() - weekly).get_days();
    assert!((56..=62).contains(&days), "{weekly} is {days} days back");
    assert_eq!(monthly, today().first_of_month() - 11.months());
}

#[test]
fn says_so_when_the_range_holds_no_usage_and_removes_the_report() {
    let sandbox = Sandbox::new("no-usage", NO_USAGE);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        run.html.as_deref(),
        Some(
            "<meta charset=\"utf-8\"><body style=\"margin:0;background:#1e1e1e\">\n\
             <pre style=\"font:13px ui-monospace,Menlo,monospace;color:#eee;padding:16px\">\n\
             No usage data.\n\
             </pre>\n"
        )
    );
    assert_eq!(run.err, "ccusage warns\n");
    sandbox.assert_report_removed();
}

#[test]
fn fails_when_quick_look_fails_removing_the_report_all_the_same() {
    let sandbox = Sandbox::new("quick-look-fails", NO_USAGE);
    sandbox.quick_look(1);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(run.html.is_some_and(|html| html.contains("No usage data.")));
    assert_eq!(run.err, "ccusage warns\nQuick Look preview failed\n");
    sandbox.assert_report_removed();
}

#[test]
fn fails_when_there_is_no_quick_look() {
    let sandbox = Sandbox::new("no-quick-look", NO_USAGE);
    fs::remove_file(sandbox.root.join("bin/qlmanage")).expect("the fake is removed");

    let run = sandbox.run_with(
        &["daily"],
        &[
            ("CLAUDE_CONFIG_DIR", sandbox.config().as_os_str()),
            ("PATH", sandbox.root.join("bin").as_os_str()),
        ],
    );

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(
        run.err.starts_with("ccusage warns\ncannot run qlmanage: "),
        "{}",
        run.err
    );
    sandbox.assert_report_removed();
}

/// One day of a project's usage as ccusage reports it, all of it on `model`.
struct Day<'a> {
    date: &'a str,
    cost: f64,
    input: u64,
    create: u64,
    read: u64,
    model: &'a str,
}

impl<'a> Day<'a> {
    fn new(date: &'a str, cost: f64) -> Self {
        Day {
            date,
            cost,
            input: 1000,
            create: 0,
            read: 0,
            model: "claude-opus-4-20250514",
        }
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "date": self.date,
            "inputTokens": self.input,
            "outputTokens": 10,
            "cacheCreationTokens": self.create,
            "cacheReadTokens": self.read,
            "totalTokens": 1010,
            "totalCost": self.cost,
            "modelBreakdowns": [{"modelName": self.model, "cost": self.cost}],
        })
    }
}

/// What ccusage prints for `projects`, each a name and its days.
fn usage(projects: &[(&str, &[Day<'_>])]) -> String {
    let projects: serde_json::Map<String, serde_json::Value> = projects
        .iter()
        .map(|(name, days)| {
            let days: Vec<_> = days.iter().map(Day::json).collect();
            ((*name).to_owned(), serde_json::Value::from(days))
        })
        .collect();
    serde_json::json!({ "projects": projects }).to_string()
}

/// Asserts that some line of `text` matches `pattern`, whose runs of spaces stand for
/// any run of spaces.
fn assert_line(text: &str, pattern: &str) {
    let words: Vec<&str> = pattern.split_whitespace().collect();
    assert!(
        text.lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == words),
        "no line reads {pattern:?} in\n{text}"
    );
}

#[test]
fn buckets_days_by_week_from_sunday_and_by_month_and_totals_each_table() {
    let days = [
        // A Saturday, then the Sunday that starts the next week.
        Day {
            read: 3000,
            model: "claude-sonnet-4-5-20250929",
            ..Day::new("2026-10-31", 1.0)
        },
        Day::new("2026-11-01", 2.5),
    ];
    let sandbox = Sandbox::new("buckets", &usage(&[("-p", &days)]));

    let weekly = sandbox
        .run(&["weekly"])
        .html
        .expect("Quick Look is reached");
    let monthly = sandbox
        .run(&["monthly"])
        .html
        .expect("Quick Look is reached");

    assert_line(
        &weekly,
        "Week Input Output Cache Create Cache Read Total Tokens Cache Hit Cost (USD)",
    );
    assert_line(&weekly, "2026-10-25 1,000 10 0 3,000 1,010 75.0% $1.00");
    assert_line(&weekly, "2026-11-01 1,000 10 0 0 1,010 0.0% $2.50");
    assert_line(&weekly, "Total 2,000 20 0 3,000 2,020 60.0% $3.50");
    assert_line(&weekly, "Week opus-4 sonnet-4-5 Total");
    assert_line(&monthly, "2026-10 1,000 10 0 3,000 1,010 75.0% $1.00");
    assert_line(&monthly, "2026-11 1,000 10 0 0 1,010 0.0% $2.50");
}

impl Sandbox {
    /// Records `cwd` as the cwd of `project`'s session log under the config dir `config`,
    /// after a line that records none.
    fn session(config: &Path, project: &str, cwd: &str) {
        let dir = config.join("projects").join(project);
        fs::create_dir_all(&dir).expect("the project's directory is created");
        fs::write(
            dir.join("s.jsonl"),
            format!(
                "{{\"type\":\"summary\"}}\n{}\n",
                serde_json::json!({ "cwd": cwd })
            ),
        )
        .expect("the session log is written");
    }

    /// A directory under the sandbox that exists, as a session's cwd.
    fn dir(&self, path: &str) -> PathBuf {
        let dir = self.root.join(path);
        fs::create_dir_all(&dir).expect("the directory is created");
        dir
    }
}

fn text(path: &Path) -> &str {
    path.to_str().expect("the sandbox's path is UTF-8")
}

#[test]
fn counts_each_project_toward_the_repository_its_sessions_ran_in() {
    let sandbox = Sandbox::new("repositories", NO_USAGE);
    let config = sandbox.config();
    let alpha = sandbox.dir("repos/alpha");
    let beta = sandbox.dir("repos/beta");
    let plain = sandbox.dir("plain");
    Sandbox::session(&config, "-alpha", text(&alpha));
    Sandbox::session(
        &config,
        "-alpha-wt",
        text(&alpha.join(".claude/worktrees/x/sub")),
    );
    Sandbox::session(
        &config,
        "-scratch",
        "/private/tmp/claude-501/-alpha/abc/scratchpad",
    );
    Sandbox::session(&config, "-beta", text(&beta));
    Sandbox::session(&config, "-moved", "/nowhere/moved-repo");
    Sandbox::session(&config, "-gone", "/tmp/gone");
    Sandbox::session(&config, "-var-gone", "/var/dotfiles-local-test-gone");
    Sandbox::session(
        &config,
        "-private-gone",
        "/private/dotfiles-local-test-gone",
    );
    Sandbox::session(&config, "-plain", text(&plain));
    let day = |cost| [Day::new("2026-11-01", cost)];
    sandbox.ccusage(
        &usage(&[
            ("-alpha", &day(1.0)),
            ("-alpha-wt", &day(2.0)),
            ("-scratch", &day(4.0)),
            ("-beta", &day(8.0)),
            ("-moved", &day(16.0)),
            ("-gone", &day(0.25)),
            ("-var-gone", &day(64.0)),
            ("-private-gone", &day(128.0)),
            ("-plain", &day(0.5)),
            ("-nolog", &day(32.0)),
        ]),
        0,
    );
    let alpha_git = alpha.join(".git");
    sandbox.repositories(&[(&alpha, text(&alpha_git)), (&beta, "/srv/git/beta.git")]);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    let html = run.html.expect("Quick Look is reached");
    let (_, by_repo) = html
        .split_once("Cost (USD) by repository - daily")
        .expect("the report has a table by repository");
    assert_line(by_repo, "Date moved-repo beta alpha other Total");
    assert_line(by_repo, "2026-11-01 $16.00 $8.00 $7.00 $224.75 $255.75");
    assert_eq!(sandbox.log("git.unexpected"), Vec::<String>::new());
}

#[test]
fn escapes_the_report_for_html() {
    let sandbox = Sandbox::new("escapes", NO_USAGE);
    let dir = sandbox.dir("r&d");
    Sandbox::session(&sandbox.config(), "-rd", text(&dir));
    let git = dir.join(".git");
    sandbox.repositories(&[(&dir, text(&git))]);
    sandbox.ccusage(
        &usage(&[(
            "-rd",
            &[Day {
                model: "<m>",
                ..Day::new("2026-11-01", 1.0)
            }],
        )]),
        0,
    );

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    assert!(html.contains("r&amp;d"), "{html}");
    assert!(html.contains("&lt;m&gt;"), "{html}");
    assert!(!html.contains("<m>"), "{html}");
}

#[test]
fn shows_a_model_name_with_multibyte_characters_near_its_end_without_the_prefix() {
    let sandbox = Sandbox::new("multibyte-model", NO_USAGE);
    let dir = sandbox.dir("repo");
    Sandbox::session(&sandbox.config(), "-repo", text(&dir));
    let git = dir.join(".git");
    sandbox.repositories(&[(&dir, text(&git))]);
    sandbox.ccusage(
        &usage(&[(
            "-repo",
            &[Day {
                model: "claude-モモモx",
                ..Day::new("2026-11-01", 1.0)
            }],
        )]),
        0,
    );

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    let html = run.html.expect("Quick Look is reached");
    let (_, by_model) = html
        .split_once("Cost (USD) by model")
        .expect("the report has a table by model");
    assert!(by_model.contains("モモモx"), "{html}");
    assert!(!by_model.contains("claude-"), "{html}");
}

#[test]
fn reads_session_logs_from_each_claude_config_dir_entry_that_holds_or_is_projects() {
    let sandbox = Sandbox::new("config-dirs", NO_USAGE);
    let holds = sandbox.root.join("holds");
    let named = sandbox.root.join("other/projects");
    let repo = sandbox.dir("repo");
    Sandbox::session(&holds, "-a", text(&repo));
    fs::create_dir_all(named.join("-b")).expect("the project's directory is created");
    fs::write(
        named.join("-b/s.jsonl"),
        format!("{}\n", serde_json::json!({ "cwd": text(&repo) })),
    )
    .expect("the session log is written");
    let git = repo.join(".git");
    sandbox.repositories(&[(&repo, text(&git))]);
    sandbox.ccusage(
        &usage(&[
            ("-a", &[Day::new("2026-11-01", 1.0)]),
            ("-b", &[Day::new("2026-11-01", 2.0)]),
        ]),
        0,
    );
    let entries = format!(
        " {} ,{},,{}/neither",
        holds.display(),
        named.display(),
        sandbox.root.display()
    );

    let run = sandbox.run_with(&["daily"], &[("CLAUDE_CONFIG_DIR", entries.as_ref())]);

    let html = run.html.expect("Quick Look is reached");
    assert_line(&html, "Date repo Total");
    assert!(!html.contains("other"), "{html}");
}

// Without CLAUDE_CONFIG_DIR, ccusage reads both of these.
#[test]
fn reads_session_logs_under_the_xdg_config_dir_and_the_home_dir_by_default() {
    let sandbox = Sandbox::new("default-dirs", NO_USAGE);
    let xdg = sandbox.root.join("xdg");
    let home = sandbox.root.join("home");
    let (first, second) = (sandbox.dir("first"), sandbox.dir("second"));
    Sandbox::session(&xdg.join("claude"), "-a", text(&first));
    Sandbox::session(&home.join(".claude"), "-b", text(&second));
    let (first_git, second_git) = (first.join(".git"), second.join(".git"));
    sandbox.repositories(&[(&first, text(&first_git)), (&second, text(&second_git))]);
    sandbox.ccusage(
        &usage(&[
            ("-a", &[Day::new("2026-11-01", 1.0)]),
            ("-b", &[Day::new("2026-11-01", 2.0)]),
        ]),
        0,
    );

    let with_xdg = sandbox.run_with(&["daily"], &[("XDG_CONFIG_HOME", xdg.as_os_str())]);
    let without = sandbox.run_with(&["daily"], &[]);

    assert_line(
        &with_xdg.html.expect("Quick Look is reached"),
        "Date second first Total",
    );
    // ~/.config/claude, where XDG_CONFIG_HOME is unset, holds no session of -a.
    assert_line(
        &without.html.expect("Quick Look is reached"),
        "Date second other Total",
    );
}

#[test]
fn fails_on_ccusage_output_with_no_per_project_breakdown() {
    let sandbox = Sandbox::new("no-projects", r#"{"totals": {"totalTokens": 5}}"#);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(1));
    assert_eq!(
        run.err,
        "ccusage warns\nccusage output has no per-project breakdown (\"projects\" key)\n"
    );
    assert_eq!(run.html, None);
}

#[test]
fn fails_at_a_day_that_lacks_a_field_the_tables_read_naming_that_day() {
    let full = Day::new("2026-11-01", 1.0).json();
    let mut broken: Vec<(&str, serde_json::Value)> = full
        .as_object()
        .expect("a day is an object")
        .keys()
        .map(|field| {
            let mut day = full.clone();
            day.as_object_mut()
                .expect("a day is an object")
                .remove(field);
            (field.as_str(), day)
        })
        .collect();
    for (field, model) in [
        ("modelName", serde_json::json!({"cost": 1.0})),
        ("cost", serde_json::json!({"modelName": "claude-opus-4"})),
    ] {
        let mut day = full.clone();
        day["modelBreakdowns"] = serde_json::json!([model]);
        broken.push((field, day));
    }
    let sandbox = Sandbox::new("lacks-a-field", NO_USAGE);

    for (field, day) in broken {
        sandbox.ccusage(
            &serde_json::json!({"projects": {"-p": [&day]}}).to_string(),
            0,
        );

        let run = sandbox.run(&["daily"]);

        assert_eq!(run.code, Some(1), "{field}: {}", run.err);
        assert!(
            run.err.contains(&format!("without \"{field}\"")),
            "{field}: {}",
            run.err
        );
        let named = if field == "modelName" || field == "cost" {
            &day["modelBreakdowns"][0]
        } else {
            &day
        };
        assert!(run.err.contains(&named.to_string()), "{field}: {}", run.err);
        assert_eq!(run.html, None, "{field}");
    }
}

#[test]
fn fails_with_ccusages_status_when_ccusage_fails() {
    let sandbox = Sandbox::new("ccusage-fails", "");
    sandbox.ccusage("", 3);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(3));
    assert!(
        run.err
            .starts_with("ccusage warns\nccusage claude daily --instances --json --since "),
        "{}",
        run.err
    );
    assert!(run.err.ends_with(" failed (exit 3)\n"), "{}", run.err);
    assert_eq!(run.html, None);
}

#[test]
fn shows_costs_to_the_nearest_cent_grouped_by_thousands_and_shares_to_a_tenth() {
    let days = [
        Day {
            input: 2000,
            read: 1000,
            ..Day::new("2026-11-01", 1234.567)
        },
        Day::new("2026-11-02", 0.129),
        // Half a cent rounds away from zero.
        Day::new("2026-11-03", 0.125),
    ];
    let sandbox = Sandbox::new("number-formats", &usage(&[("-p", &days)]));

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    assert_line(&html, "2026-11-01 2,000 10 0 1,000 1,010 33.3% $1,234.57");
    assert_line(&html, "2026-11-02 1,000 10 0 0 1,010 0.0% $0.13");
    assert_line(&html, "2026-11-03 1,000 10 0 0 1,010 0.0% $0.13");
    assert_line(&html, "Total 4,000 30 0 1,000 3,030 20.0% $1,234.82");
}

impl Sandbox {
    /// Runs `args` and asserts that the run failed with exit 1 and `message` as its one
    /// line on stderr, before reaching Quick Look.
    fn assert_fails(&self, args: &[&str], message: &str) {
        let run = self.run(args);
        assert_eq!(
            (run.code, run.err.as_str(), run.html),
            (
                Some(1),
                format!("ccusage warns\n{message}\n").as_str(),
                None
            )
        );
    }
}

#[test]
fn fails_on_ccusage_output_that_is_not_json() {
    let sandbox = Sandbox::new("not-json", "not json");

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(
        run.err
            .starts_with("ccusage warns\nccusage printed no JSON: "),
        "{}",
        run.err
    );
    assert_eq!(run.err.lines().count(), 2, "{}", run.err);
}

#[test]
fn fails_on_projects_that_are_not_an_object() {
    let sandbox = Sandbox::new("projects-list", r#"{"projects": []}"#);

    sandbox.assert_fails(
        &["daily"],
        "ccusage output has no per-project breakdown (\"projects\" key)",
    );
}

#[test]
fn fails_on_a_project_whose_days_are_not_a_list() {
    let sandbox = Sandbox::new("days-object", r#"{"projects": {"-p": {"a": 1}}}"#);

    sandbox.assert_fails(
        &["daily"],
        r#"ccusage reported no list of days for -p: {"a":1}"#,
    );
}

#[test]
fn fails_on_a_date_a_week_cannot_be_found_for() {
    let day = Day::new("x", 1.0);
    let sandbox = Sandbox::new("bad-date", &usage(&[("-p", &[day])]));

    sandbox.assert_fails(
        &["weekly"],
        &format!(
            "ccusage reported usage whose \"date\" is no date: {}",
            Day::new("x", 1.0).json()
        ),
    );
}

#[test]
fn fails_when_there_is_no_ccusage() {
    let sandbox = Sandbox::new("no-ccusage", NO_USAGE);
    fs::remove_file(sandbox.root.join("bin/ccusage")).expect("the fake is removed");

    let run = sandbox.run_with(
        &["daily"],
        &[
            ("CLAUDE_CONFIG_DIR", sandbox.config().as_os_str()),
            ("PATH", sandbox.root.join("bin").as_os_str()),
        ],
    );

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(run.err.starts_with("cannot run ccusage: "), "{}", run.err);
}

#[test]
fn fails_on_a_session_log_it_cannot_read() {
    let sandbox = Sandbox::new("unreadable-log", NO_USAGE);
    Sandbox::session(&sandbox.config(), "-p", "/nowhere");
    let log = sandbox.config().join("projects/-p/s.jsonl");
    fs::set_permissions(&log, fs::Permissions::from_mode(0o000))
        .expect("the log is made unreadable");
    sandbox.ccusage(&usage(&[("-p", &[Day::new("2026-11-01", 1.0)])]), 0);

    let run = sandbox.run(&["daily"]);

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(
        run.err
            .starts_with(&format!("ccusage warns\ncannot read {}: ", log.display())),
        "{}",
        run.err
    );
}

/// The whole report for two days on two models, one of them named with a character of
/// more than one byte, each model on one of the days alone.
const TWO_MODELS: &str = "\
Claude Code usage - daily

Date        Input  Output  Cache Create  Cache Read  Total Tokens  Cache Hit  Cost (USD)
----------  -----  ------  ------------  ----------  ------------  ---------  ----------
2026-11-01  1,000      10             0           0         1,010       0.0%       $1.50
2026-11-02  1,000      10             0           0         1,010       0.0%      $12.25
----------  -----  ------  ------------  ----------  ------------  ---------  ----------
Total       2,000      20             0           0         2,020       0.0%      $13.75

Cost (USD) by model - daily

Date        opus-4  \u{fc}-model   Total
----------  ------  -------  ------
2026-11-01            $1.50   $1.50
2026-11-02  $12.25           $12.25
----------  ------  -------  ------
Total       $12.25    $1.50  $13.75

Cost (USD) by repository - daily

Date         other   Total
----------  ------  ------
2026-11-01   $1.50   $1.50
2026-11-02  $12.25  $12.25
----------  ------  ------
Total       $13.75  $13.75";

// Quick Look shows the report in a monospace font, so the columns line up as they are
// padded here, a blank cell where a key has no cost that day.
#[test]
fn lays_out_each_table_in_aligned_columns() {
    let days = [
        Day {
            model: "claude-\u{fc}-model",
            ..Day::new("2026-11-01", 1.5)
        },
        Day::new("2026-11-02", 12.25),
    ];
    let sandbox = Sandbox::new("layout", &usage(&[("-p", &days)]));

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    let report = html
        .split_once(";padding:16px\">\n")
        .and_then(|(_, rest)| rest.strip_suffix("\n</pre>\n"))
        .expect("the report sits in the <pre>");
    assert_eq!(report, TWO_MODELS);
}

#[test]
fn keeps_a_model_name_whose_last_nine_characters_are_not_a_date() {
    let days = [
        Day {
            model: "claude-opus-4-2025051x",
            ..Day::new("2026-11-01", 1.0)
        },
        Day {
            model: "claude-opus-4-latest-1",
            ..Day::new("2026-11-01", 2.0)
        },
    ];
    let sandbox = Sandbox::new("not-a-date", &usage(&[("-p", &days)]));

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    assert_line(&html, "Date opus-4-latest-1 opus-4-2025051x Total");
}

// A scratchpad's path names its uid in digits; a path without them is no scratchpad,
// even where its next component names a project.
#[test]
fn counts_a_session_toward_its_parent_project_only_under_a_uid() {
    let sandbox = Sandbox::new("scratchpad-uid", NO_USAGE);
    let config = sandbox.config();
    let repo = sandbox.dir("repo");
    Sandbox::session(&config, "-repo", text(&repo));
    Sandbox::session(
        &config,
        "-uid",
        "/private/tmp/claude-501/-repo/s/scratchpad",
    );
    Sandbox::session(
        &config,
        "-no-uid",
        "/private/tmp/claude-/-repo/s/scratchpad",
    );
    let git = repo.join(".git");
    sandbox.repositories(&[(&repo, text(&git))]);
    sandbox.ccusage(
        &usage(&[
            ("-uid", &[Day::new("2026-11-01", 1.0)]),
            ("-no-uid", &[Day::new("2026-11-01", 2.0)]),
        ]),
        0,
    );

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    let (_, by_repo) = html
        .split_once("Cost (USD) by repository - daily")
        .expect("the report has a table by repository");
    assert_line(by_repo, "Date repo other Total");
    assert_line(by_repo, "2026-11-01 $1.00 $2.00 $3.00");
}

// A project's sessions are its *.jsonl logs, read in name order until one records a cwd.
#[test]
fn takes_a_projects_cwd_from_the_first_jsonl_log_by_name_that_records_one() {
    let sandbox = Sandbox::new("first-log", NO_USAGE);
    let (repo, wrong) = (sandbox.dir("repo"), sandbox.dir("wrong"));
    let dir = sandbox.config().join("projects/-p");
    fs::create_dir_all(&dir).expect("the project's directory is created");
    let cwd = |path: &Path| format!("{}\n", serde_json::json!({ "cwd": text(path) }));
    for (name, content) in [
        ("0.txt", cwd(&wrong)),
        (".0.jsonl", cwd(&wrong)),
        ("a.jsonl", "{\"type\":\"summary\"}\n".to_owned()),
        ("b.jsonl", cwd(&repo)),
    ] {
        fs::write(dir.join(name), content).expect("the log is written");
    }
    // Many logs after it, so that an order other than by name is all but sure to reach
    // one of them first.
    for name in 'c'..='z' {
        fs::write(dir.join(format!("{name}.jsonl")), cwd(&wrong)).expect("the log is written");
    }
    let (repo_git, wrong_git) = (repo.join(".git"), wrong.join(".git"));
    sandbox.repositories(&[(&repo, text(&repo_git)), (&wrong, text(&wrong_git))]);
    sandbox.ccusage(&usage(&[("-p", &[Day::new("2026-11-01", 1.0)])]), 0);

    let html = sandbox.run(&["daily"]).html.expect("Quick Look is reached");

    assert_line(&html, "Date repo Total");
}
