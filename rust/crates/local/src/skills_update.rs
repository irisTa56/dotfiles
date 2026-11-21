//! `skills-update`: updates APM skills, holding back any whose last change upstream
//! merged under a day ago.
//!
//! It reads and updates the apm.yml and apm.lock.yaml of the directory it runs in,
//! which the mise task calling it leaves as the root of the checkout it ran in.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Stdio;

use clap::Args;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::apm_lock::{ApmLock, LockedDependency};
use crate::failure::Failure;
use crate::program::{self, Program};

/// Update APM skills, holding back any whose last change upstream merged under a day ago.
///
/// Updates APM skills, every one or those named, holding back any whose last change
/// upstream merged less than a day ago, as the release windows set for npm, uv and Hex
/// do. APM has no such window of its own. Run it as `mise run skills:update`.
///
/// The time comes from the pull request that merged the skill's last commit into the
/// default branch, which GitHub records, since a commit's own date is whatever its
/// author set. A commit GitHub lists no such pull request for is held too, though the
/// list can miss one, so that hold is for checking by hand. `apm update <package>`
/// followed by `apm install` is the way past a hold.
#[derive(Debug, Args)]
pub(crate) struct Request {
    /// The skills to update, as `apm update` names them [default: every one]. Every
    /// argument after the first is one, even one that opens with `-`
    #[arg(trailing_var_arg = true)]
    packages: Vec<String>,
}

/// How long after its last change merged upstream a skill is held back.
const WINDOW: SignedDuration = SignedDuration::from_hours(24);

impl Request {
    /// Updates the skills the request names, every one where it names none.
    pub(crate) fn run(self) -> Result<(), Failure> {
        update(&self.packages)
    }
}

/// Holds back what changed too recently, and has apm update the rest.
fn update(packages: &[String]) -> Result<(), Failure> {
    let lockfile = Lockfile::read()?;
    let chosen = lockfile.chosen(packages)?;
    let mut ready = Vec::new();
    let mut held = Vec::new();
    for dependency in chosen {
        match dependency.hold()? {
            None => ready.push(dependency),
            Some(reason) => held.push((dependency, reason)),
        }
    }
    let mut stdout = io::stdout();
    for (dependency, reason) in &held {
        let _ = writeln!(stdout, "{}: held, {reason}", dependency.name);
    }
    if ready.is_empty() {
        let _ = writeln!(stdout, "Nothing to update.");
        return Ok(());
    }
    let mut update = vec!["update".to_owned(), "--yes".to_owned()];
    update.extend(ready.iter().map(Dependency::key));
    apm(&update)?;
    // `apm update` can leave deployed files removed, which only this restores.
    apm(&["install".to_owned()])?;
    apm(&["audit".to_owned()])
}

/// Runs apm with `args`, its output and errors both passed on to stdout.
///
/// They reach stdout through a pipe rather than as stdout itself, so apm writes as it
/// does for a reader that is not a terminal, without colors.
fn apm(args: &[String]) -> Result<(), Failure> {
    let label = format!("apm {} failed", args.join(" "));
    let failed = |error: io::Error| Failure::new(format_args!("{label} ({error})"), 1);
    let (mut reader, writer) = io::pipe().map_err(failed)?;
    let errors = writer.try_clone().map_err(failed)?;
    // The command, holding this side's write ends of the pipe, is dropped at the end of
    // this statement, so the reading below ends once apm has closed its own.
    let mut child = Program::new("apm")
        .args(args)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(errors)
        .spawn()?;
    pass_on(&mut reader);
    let status = child.wait().map_err(failed)?;
    if status.success() {
        return Ok(());
    }
    Err(program::failed(status, format_args!("{label}")))
}

/// Copies what `reader` yields to stdout until it ends, dropping a write that fails, so
/// that apm is never left blocked on a pipe no one reads.
fn pass_on(reader: &mut impl Read) {
    let mut stdout = io::stdout();
    let mut buffer = [0; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => {
                let _ = stdout
                    .write_all(&buffer[..read])
                    .and_then(|()| stdout.flush());
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

/// The dependencies apm.lock.yaml lists.
struct Lockfile {
    dependencies: Vec<Dependency>,
}

impl Lockfile {
    /// Reads apm.lock.yaml in the directory the run is in.
    fn read() -> Result<Self, Failure> {
        let lock = ApmLock::read(Path::new("."))?;
        let Some(entries) = lock.dependencies() else {
            return Err(Failure::new(
                format_args!("apm.lock.yaml has no dependencies list: {lock}"),
                1,
            ));
        };
        let dependencies = entries
            .iter()
            .map(Dependency::from_entry)
            .collect::<Result<_, _>>()?;
        Ok(Lockfile { dependencies })
    }

    /// The dependencies `packages` name, in the lockfile's order, or every one where they
    /// name none.
    ///
    /// Each dependency takes one name off the list of those given, so a name given twice
    /// leaves one behind and is refused.
    fn chosen(self, packages: &[String]) -> Result<Vec<Dependency>, Failure> {
        let mut unknown: Vec<&str> = packages.iter().map(String::as_str).collect();
        for dependency in &self.dependencies {
            let key = dependency.key();
            if let Some(index) = unknown.iter().position(|package| *package == key) {
                unknown.remove(index);
            }
        }
        if !unknown.is_empty() {
            return Err(Failure::new(
                format_args!("Not in apm.lock.yaml: {}", unknown.join(" ")),
                1,
            ));
        }
        Ok(self
            .dependencies
            .into_iter()
            .filter(|dependency| {
                packages.is_empty() || packages.iter().any(|package| *package == dependency.key())
            })
            .collect())
    }
}

/// One dependency of the lockfile.
struct Dependency {
    name: String,
    /// Where it comes from: a gist, or a repository GitHub is asked about.
    source: Source,
    /// The path to it within its repository, absent for a whole repository.
    virtual_path: Option<String>,
}

enum Source {
    /// A gist, the owner's own, which `skills:push` moves.
    Gist {
        repo_url: Option<String>,
    },
    Repository {
        repo_url: String,
    },
}

impl Dependency {
    /// The dependency an entry of the lockfile describes.
    ///
    /// One that is not a gist names its repository, so one without it fails here rather
    /// than as a request for no repository.
    fn from_entry(entry: &LockedDependency) -> Result<Self, Failure> {
        let malformed = |what: &str| {
            Failure::new(
                format_args!("a dependency in apm.lock.yaml has no {what}: {entry}"),
                1,
            )
        };
        let name = entry.name().ok_or_else(|| malformed("name"))?.to_owned();
        let repo_url = entry.repo_url().map(str::to_owned);
        let source = if entry.host() == Some("gist.github.com") {
            Source::Gist { repo_url }
        } else {
            Source::Repository {
                repo_url: repo_url.ok_or_else(|| malformed("repository"))?,
            }
        };
        Ok(Dependency {
            name,
            source,
            virtual_path: entry.virtual_path().map(str::to_owned),
        })
    }

    /// What `apm update` takes: the path into the repository, or the name.
    fn key(&self) -> String {
        let repo_url = match &self.source {
            Source::Gist { repo_url } => repo_url.as_deref(),
            Source::Repository { repo_url } => Some(repo_url.as_str()),
        };
        match (repo_url, &self.virtual_path) {
            (Some(repo_url), Some(path)) => format!("{repo_url}/{path}"),
            _ => self.name.clone(),
        }
    }

    /// Why this dependency is held back, or `None` where it is not.
    fn hold(&self) -> Result<Option<String>, Failure> {
        let Source::Repository { repo_url } = &self.source else {
            return Ok(None);
        };
        let query = self
            .virtual_path
            .as_ref()
            .map_or_else(String::new, |path| format!("path={path}&"));
        let sha = match (GithubQuery {
            path: format!("repos/{repo_url}/commits?{query}per_page=1"),
            pick: last_commit,
            when_empty: "no commit on this path",
        })
        .answer()?
        {
            Ok(sha) => sha,
            Err(reason) => return Ok(Some(reason)),
        };
        let merged = match (GithubQuery {
            path: format!("repos/{repo_url}/commits/{sha}/pulls"),
            pick: merged_into_default,
            when_empty: "GitHub lists no merge into the default branch for its last commit; check by hand",
        })
        .answer()?
        {
            Ok(merged) => merged,
            Err(reason) => return Ok(Some(reason)),
        };
        let Ok(time) = merged.parse::<Timestamp>() else {
            return Ok(Some(format!(
                "GitHub reported its last merge at {merged:?}, not a timestamp"
            )));
        };
        Ok((Timestamp::now().duration_since(time) < WINDOW)
            .then(|| format!("last change merged {merged}")))
    }
}

/// A request to GitHub's API through gh, what to take from its answer, and what there
/// being nothing to take means.
struct GithubQuery {
    path: String,
    pick: fn(&Value) -> Option<String>,
    when_empty: &'static str,
}

impl GithubQuery {
    /// The answer, or the reason to hold where there is none; fails where gh cannot run.
    fn answer(&self) -> Result<Result<String, String>, Failure> {
        let output = Program::new("gh")
            .args(["api", &self.path])
            .stderr(Stdio::inherit())
            .output()?;
        if !output.status.success() {
            return Ok(Err(format!(
                "GitHub request failed (exit {})",
                program::exit_code(output.status)
            )));
        }
        let answer: Value = match serde_json::from_slice(&output.stdout) {
            Ok(answer) => answer,
            Err(error) => return Ok(Err(format!("GitHub's answer is not JSON: {error}"))),
        };
        Ok((self.pick)(&answer).ok_or_else(|| self.when_empty.to_owned()))
    }
}

/// The commit GitHub lists first, its latest, from an answer listing commits.
fn last_commit(commits: &Value) -> Option<String> {
    commits.get(0)?.get("sha")?.as_str().map(str::to_owned)
}

/// The latest time one of the pull requests GitHub lists for a commit merged it into the
/// default branch. GitHub can list only the pull request that took a commit into
/// another branch, whose merge time says nothing of when the commit reached the default
/// one.
fn merged_into_default(pulls: &Value) -> Option<String> {
    pulls
        .as_array()?
        .iter()
        .filter(|pull| pull.pointer("/base/ref") == pull.pointer("/base/repo/default_branch"))
        .filter_map(|pull| pull.get("merged_at")?.as_str())
        .max()
        .map(str::to_owned)
}
