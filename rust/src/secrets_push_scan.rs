//! `secrets:push-scan`: whether the commits a push is about to send hold a secret, as
//! trufflehog finds one.
//!
//! `secrets:commit-scan` gates each commit with gitleaks, and this is the second pass its
//! default rules do not give, paid once per push rather than once per commit. A pre-push
//! hook is the one caller it works for: git names the remote in the hook's first argument
//! and feeds it one line per pushed ref on stdin. A caller that passes the argument and no
//! ref lines cannot be told apart from an up-to-date push, which git runs the hook for with
//! an empty stdin, so it scans nothing and passes.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt;
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{ExitCode, Output, Stdio};

use clap::Args;
use serde_json::Value;

use crate::git::Git;
use crate::program::Program;
use crate::stream::Stream;

/// The status of a push this stops, whether for a secret found or for a scan that could not
/// answer.
const STOPPED: u8 = 1;

/// The arguments git runs a pre-push hook with: the remote's name, and its URL, which the
/// scan does not read.
#[derive(Debug, Args)]
pub(crate) struct Hook {
    /// The name of the remote the push goes to.
    remote: String,
    /// The remote's URL, which the scan does not read.
    url: Option<String>,
}

/// A push about to be sent from the repository the task runs in.
#[derive(Debug)]
pub(crate) struct Push {
    /// The name of the remote the push goes to.
    remote: String,
    /// The repository as trufflehog reads it: a `file://` URL to its root, not the working
    /// directory, for the reason `secrets:commit-scan` gives.
    url: String,
}

/// What a push sends of one ref.
#[derive(Debug)]
struct Sending {
    /// The commits the remote does not hold yet.
    commits: HashSet<String>,
    /// The commit git is pushing, peeled so that an annotated tag gives its commit. The walk
    /// starts here rather than at the ref's name, which trufflehog would resolve again in its
    /// own clone, where a tag of the same name wins over the branch.
    tip: String,
}

/// trufflehog's run over the history behind one commit: its `--json` report on stdout,
/// and its log on stderr.
#[derive(Debug)]
struct Report(Output);

/// A secret trufflehog found in a commit the push sends.
#[derive(Debug)]
struct Hit {
    detector: String,
    line: String,
    /// None where the secret is in the commit's message.
    file: Option<String>,
    commit: String,
}

impl Push {
    /// Scans what the push sends, `hook` being the hook's arguments and stdin its ref
    /// lines, and returns the code to exit with.
    pub(crate) fn scan(hook: &Hook) -> ExitCode {
        match Self::stages(hook) {
            Ok(code) | Err(code) => code,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a code it has already
    /// reported.
    fn stages(hook: &Hook) -> Result<ExitCode, ExitCode> {
        let push = Push::from_hook(hook)?;
        let mut found = false;
        for local in Self::pushed()? {
            let Some(sending) = push.sending(&local)? else {
                continue;
            };
            let hits = push.hits(&sending)?;
            for hit in &hits {
                Stream::Stderr.line(format_args!("{hit}"));
            }
            found |= !hits.is_empty();
        }
        if !found {
            return Ok(ExitCode::SUCCESS);
        }
        // Said once, after every ref's hits: the marker that clears a false positive clears
        // a real secret too, so a push made on someone's behalf asks them first.
        Stream::Stderr.line(format_args!(
            "A trufflehog:ignore comment on a reported line lets it through only when the commit that introduced the line carries it, so adding one means rewriting that commit; it lets a real secret through as readily, so an agent pushing for someone shows them these hits and rewrites only on their confirmation."
        ));
        Ok(ExitCode::from(STOPPED))
    }

    /// The push the hook's arguments name, from the repository the environment a git hook
    /// exports names.
    fn from_hook(hook: &Hook) -> Result<Self, ExitCode> {
        let top = Git::inheriting().run(&["rev-parse", "--show-toplevel"]);
        if !top.success() {
            return Err(top.failure(STOPPED));
        }
        let line = top.stdout_bytes();
        let root = Path::new(OsStr::from_bytes(line.strip_suffix(b"\n").unwrap_or(line)));
        Ok(Push {
            remote: hook.remote.clone(),
            url: format!("file://{}", root.display()),
        })
    }

    /// The local object of each ref the push sends, read from the lines git feeds the hook,
    /// `<local ref> <local sha> <remote ref> <remote sha>`. A ref's name may hold any bytes
    /// git allows, so the lines are split as bytes. A deletion arrives with an all-zero local
    /// sha, and adds nothing.
    fn pushed() -> Result<Vec<String>, ExitCode> {
        let mut input = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut input)
            .map_err(|error| {
                Stream::Stderr.line(format_args!("could not read the pushed refs: {error}"));
                ExitCode::from(STOPPED)
            })?;
        Ok(input
            .split(|&byte| byte == b'\n')
            .filter_map(|line| {
                line.split(u8::is_ascii_whitespace)
                    .filter(|field| !field.is_empty())
                    .nth(1)
            })
            .filter(|local| local.iter().any(|&byte| byte != b'0'))
            .map(|local| String::from_utf8_lossy(local).into_owned())
            .collect())
    }

    /// What the push sends of the ref at `local`, or None where the remote holds all of it
    /// already, and scanning would answer for what it already has.
    fn sending(&self, local: &str) -> Result<Option<Sending>, ExitCode> {
        let git = Git::inheriting();
        let remotes = format!("--remotes={}", self.remote);
        let new = git.run(&["rev-list", local, "--not", &remotes]);
        if !new.success() {
            return Err(new.failure(STOPPED));
        }
        let commits: HashSet<String> = new.stdout().lines().map(str::to_owned).collect();
        if commits.is_empty() {
            return Ok(None);
        }
        let tip = git.run(&["rev-parse", &format!("{local}^{{commit}}")]);
        if !tip.success() {
            return Err(tip.failure(STOPPED));
        }
        Ok(Some(Sending {
            commits,
            tip: tip.stdout().trim_end().to_owned(),
        }))
    }

    /// The secrets trufflehog finds in the commits `sending` holds.
    fn hits(&self, sending: &Sending) -> Result<Vec<Hit>, ExitCode> {
        self.trufflehog(&sending.tip)?.hits_in(sending)
    }

    /// trufflehog's report on the whole history behind `tip`, from a scan that completed.
    ///
    /// `--trust-local-git-config` is left off: with it trufflehog reads the repository with
    /// go-git, which rejects a config that sets `extensions.worktreeConfig`. Verification
    /// stays off so a push never sends a candidate to its provider.
    fn trufflehog(&self, tip: &str) -> Result<Report, ExitCode> {
        let output = Program::new("trufflehog")
            .args(["git", &self.url, "--branch", tip])
            .args([
                "--json",
                "--no-verification",
                "--fail-on-scan-errors",
                "--no-update",
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| {
                Stream::Stderr.line(format_args!("{error}"));
                ExitCode::from(STOPPED)
            })?;
        let report = Report(output);
        if !report.0.status.success() {
            return Err(report.unanswered(None));
        }
        Ok(report)
    }
}

impl Report {
    /// The hits in the commits `sending` holds; trufflehog walked the history behind them
    /// too. A result that names no commit cannot be told to be in them or not, and passing
    /// it over would let a push through on a report this cannot read.
    fn hits_in(&self, sending: &Sending) -> Result<Vec<Hit>, ExitCode> {
        let mut hits = Vec::new();
        for line in String::from_utf8_lossy(&self.0.stdout).lines() {
            if line.is_empty() {
                continue;
            }
            let result: Value =
                serde_json::from_str(line).map_err(|error| self.unanswered(Some(&error)))?;
            let hit = Hit::from_result(&result)
                .ok_or_else(|| self.unanswered(Some(&"a result names no commit")))?;
            if sending.commits.contains(&hit.commit) {
                hits.push(hit);
            }
        }
        Ok(hits)
    }

    /// Reports a scan that gave no answer, and returns the code to end with. trufflehog's
    /// log is held back otherwise, since it counts every hit it walked and so would announce
    /// a secret on a push the filter lets through; here it is the one account of why.
    fn unanswered(&self, reason: Option<&dyn fmt::Display>) -> ExitCode {
        Stream::Stderr.bytes(&self.0.stderr);
        if let Some(reason) = reason {
            Stream::Stderr.line(format_args!("could not read trufflehog's report: {reason}"));
        }
        ExitCode::from(STOPPED)
    }
}

impl Hit {
    /// The hit one line of trufflehog's `--json` report describes, or None where it names
    /// no commit.
    fn from_result(result: &Value) -> Option<Self> {
        let git = &result["SourceMetadata"]["Data"]["Git"];
        Some(Hit {
            detector: text(&result["DetectorName"]),
            line: text(&git["line"]),
            file: git["file"].as_str().map(str::to_owned),
            commit: git["commit"].as_str()?.to_owned(),
        })
    }
}

impl fmt::Display for Hit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {} of {}, commit {}",
            self.detector,
            self.line,
            self.file.as_deref().unwrap_or("the commit message"),
            self.commit
        )
    }
}

/// A field of trufflehog's report as text: a string as it is, anything else as JSON writes it.
fn text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}
