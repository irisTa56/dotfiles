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
use std::io::{self, BufRead};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use serde_json::Value;

use crate::git::Git;
use crate::stream::Stream;

/// The status of a push this stops, whether for a secret found or for a scan that could not
/// answer.
const STOPPED: u8 = 1;

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
    /// Scans what the push sends, `args` being the hook's arguments and stdin its ref lines,
    /// and returns the code to exit with.
    pub(crate) fn scan(args: &[String]) -> ExitCode {
        match Self::stages(args) {
            Ok(code) | Err(code) => code,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a code it has already
    /// reported.
    fn stages(args: &[String]) -> Result<ExitCode, ExitCode> {
        let push = Push::from_hook(args)?;
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
    fn from_hook(args: &[String]) -> Result<Self, ExitCode> {
        let Some(remote) = args.first() else {
            Stream::Stderr.line(format_args!(
                "usage: dotfiles-tasks secrets-push-scan <remote>, as git runs a pre-push hook: the remote's name as the first argument and the pushed refs on stdin"
            ));
            return Err(ExitCode::from(STOPPED));
        };
        let top = Git::inheriting().run(&["rev-parse", "--show-toplevel"]);
        if !top.success() {
            return Err(top.failure(STOPPED));
        }
        let line = top.stdout_bytes();
        let root = Path::new(OsStr::from_bytes(line.strip_suffix(b"\n").unwrap_or(line)));
        Ok(Push {
            remote: remote.clone(),
            url: format!("file://{}", root.display()),
        })
    }

    /// The local object of each ref the push sends, read from the lines git feeds the hook,
    /// `<local ref> <local sha> <remote ref> <remote sha>`. A deletion arrives with an
    /// all-zero local sha, and adds nothing.
    fn pushed() -> Result<Vec<String>, ExitCode> {
        let mut pushed = Vec::new();
        for line in io::stdin().lock().lines() {
            let line = line.map_err(|error| {
                Stream::Stderr.line(format_args!("could not read the pushed refs: {error}"));
                ExitCode::from(STOPPED)
            })?;
            if let Some(local) = line.split_whitespace().nth(1)
                && local.bytes().any(|byte| byte != b'0')
            {
                pushed.push(local.to_owned());
            }
        }
        Ok(pushed)
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

    /// The secrets trufflehog finds in the commits `sending` holds. It walks the whole
    /// history behind the tip, and only the hits in those commits are kept.
    ///
    /// `--trust-local-git-config` is left off: with it trufflehog reads the repository with
    /// go-git, which rejects a config that sets `extensions.worktreeConfig`. Verification
    /// stays off so a push never sends a candidate to its provider.
    ///
    /// trufflehog's log is held back, since it counts every hit it walked and so would
    /// announce a secret on a push the filter lets through, and is printed where the scan
    /// failed, the one time it says something this cannot.
    fn hits(&self, sending: &Sending) -> Result<Vec<Hit>, ExitCode> {
        let output = Command::new("trufflehog")
            .args(["git", &self.url, "--branch", &sending.tip])
            .args([
                "--json",
                "--no-verification",
                "--fail-on-scan-errors",
                "--no-update",
            ])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| {
                Stream::Stderr.line(format_args!("could not run trufflehog: {error}"));
                ExitCode::from(STOPPED)
            })?;
        let unanswered = |reason: Option<&dyn fmt::Display>| {
            Stream::Stderr.bytes(&output.stderr);
            if let Some(reason) = reason {
                Stream::Stderr.line(format_args!("could not read trufflehog's report: {reason}"));
            }
            ExitCode::from(STOPPED)
        };
        if !output.status.success() {
            return Err(unanswered(None));
        }
        let mut hits = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.is_empty() {
                continue;
            }
            let result: Value =
                serde_json::from_str(line).map_err(|error| unanswered(Some(&error)))?;
            let hit = Hit::from_result(&result);
            if sending.commits.contains(&hit.commit) {
                hits.push(hit);
            }
        }
        Ok(hits)
    }
}

impl Hit {
    /// The hit one line of trufflehog's `--json` report describes.
    fn from_result(result: &Value) -> Self {
        let git = &result["SourceMetadata"]["Data"]["Git"];
        Hit {
            detector: text(&result["DetectorName"]),
            line: text(&git["line"]),
            file: git["file"].as_str().map(str::to_owned),
            commit: text(&git["commit"]),
        }
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
