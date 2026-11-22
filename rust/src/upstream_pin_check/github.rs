//! Opening a pull request through GitHub's REST API, with curl.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;

use dotfiles_shared::Failure;

use super::{FailLine, GitHub, PullRequest};
use crate::program::Program;

/// Where GitHub Actions leaves `GITHUB_API_URL` unset, which it does not on GitHub.com.
const DEFAULT_API: &str = "https://api.github.com";
/// How long curl waits to connect, and for the whole request, in seconds, so that a
/// stalled API ends the run rather than holding the job until its own timeout.
const CONNECT_TIMEOUT: &str = "10";
const MAX_TIME: &str = "60";

impl GitHub {
    /// Reads what opening a pull request needs from the step's environment, and checks
    /// that curl runs, so that a run lacking either stops before it pushes anything.
    pub(super) fn from_env() -> Result<Self, Failure> {
        let token = env::var("GITHUB_TOKEN").unwrap_or_default();
        if token.is_empty() {
            return Err(FailLine("opening the pull request needs the job's token in the step's env, as GITHUB_TOKEN: ${{ github.token }}").ending());
        }
        let repository = env::var("GITHUB_REPOSITORY").unwrap_or_default();
        if repository.is_empty() {
            return Err(FailLine(
                "opening the pull request needs GITHUB_REPOSITORY, which GitHub Actions sets",
            )
            .ending());
        }
        let curl = Program::new("curl")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !curl.is_ok_and(|status| status.success()) {
            return Err(FailLine("opening the pull request needs curl on PATH").ending());
        }
        let api = env::var("GITHUB_API_URL")
            .ok()
            .filter(|api| !api.is_empty())
            .unwrap_or_else(|| DEFAULT_API.to_owned());
        Ok(GitHub {
            api,
            repository,
            token,
        })
    }

    /// Opens `pull`, and returns its page, or what GitHub or curl said where it could not.
    /// The token goes to curl in a header file under `dir`, so that it is on no command
    /// line.
    pub(super) fn open(&self, dir: &Path, pull: &PullRequest) -> Result<String, String> {
        let header = dir.join("github-authorization");
        fs::write(&header, format!("Authorization: Bearer {}\n", self.token))
            .map_err(|error| format!("could not write the token's header file: {error}"))?;
        let mut body = serde_json::json!({
            "head": pull.head,
            "base": pull.base,
            "title": pull.title,
            "body": pull.body,
        });
        // Only where asked for, so that any other request is the one GitHub always took,
        // whatever a repository's plan says of drafts.
        if pull.draft {
            body["draft"] = serde_json::Value::Bool(true);
        }
        let output = Program::new("curl")
            .args(["-sS", "-X", "POST"])
            .args(["--connect-timeout", CONNECT_TIMEOUT, "--max-time", MAX_TIME])
            .arg("-H")
            .arg(format!("@{}", header.display()))
            .args(["-H", "Accept: application/vnd.github+json"])
            .args(["-H", "X-GitHub-Api-Version: 2022-11-28"])
            .args(["--data-binary", &body.to_string()])
            .args(["--write-out", "\n%{http_code}"])
            .arg(format!("{}/repos/{}/pulls", self.api, self.repository))
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (answer, code) = stdout.rsplit_once('\n').unwrap_or(("", &stdout));
        let answer: serde_json::Value =
            serde_json::from_str(answer).unwrap_or(serde_json::Value::Null);
        match code.trim() {
            "201" => answer["html_url"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "GitHub's answer named no pull request".to_owned()),
            code => Err(refusal(code, &answer)),
        }
    }
}

/// What GitHub said in refusing, with `HTTP code`: its message, each validation error's,
/// and where the refusal is one of permission, what grants it.
fn refusal(code: &str, answer: &serde_json::Value) -> String {
    let mut reason = format!(
        "GitHub refused it with HTTP {code}: {}",
        answer["message"].as_str().unwrap_or("no reason given")
    );
    for error in answer["errors"].as_array().into_iter().flatten() {
        if let Some(message) = error["message"].as_str() {
            reason.push_str("; ");
            reason.push_str(message);
        }
    }
    if code == "401" || code == "403" {
        reason.push_str("; opening one needs the job's token with pull-requests: write, and the repository setting \"Allow GitHub Actions to create and approve pull requests\"");
    }
    reason
}
