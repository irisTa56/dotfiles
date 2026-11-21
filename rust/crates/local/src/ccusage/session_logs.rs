//! Which repository each project counts toward, from the session logs ccusage reads.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::failure::Failure;

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
pub(super) struct SessionLogs {
    /// The projects directories ccusage reads.
    dirs: Vec<PathBuf>,
}

impl SessionLogs {
    /// The projects directories ccusage reads. Each comma-separated `CLAUDE_CONFIG_DIR`
    /// entry counts when it holds `projects/` or is itself named `projects`. Without the
    /// variable, ccusage reads `projects/` under `$XDG_CONFIG_HOME/claude`
    /// (`~/.config/claude` when that is unset) and under `~/.claude`.
    pub(super) fn from_env() -> Result<Self, Failure> {
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
    pub(super) fn cwd(&self, project: &str) -> Result<Option<String>, Failure> {
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
    pub(super) fn repo_label(&self, cwd: Option<&str>) -> Result<String, Failure> {
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

/// The parent project of a session started in a Claude scratchpad.
fn scratchpad_parent(cwd: &str) -> Option<&str> {
    static SCRATCHPAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("^(?:/private)?/tmp/claude-[0-9]+/([^/]+)/").expect("the pattern compiles")
    });
    Some(SCRATCHPAD.captures(cwd)?.get(1)?.as_str())
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
