//! `links:commit-check`: whether the links of the files a commit stages resolve, as
//! lychee checks them under the repository's own lychee config.
//!
//! Every link in a staged file is requested, old ones included, so a host failing on one
//! of those blocks the commit too; a link in an unstaged file blocks nothing. `--cache`
//! skips links that passed recently, and lychee never caches a failure.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::{ExitCode, ExitStatus, Stdio};

use crate::git::Git;
use crate::program::Program;
use crate::stream::Stream;

/// The status of a run that could not check the links, where neither git nor lychee
/// gave one.
const UNCHECKED: u8 = 2;

/// The most bytes of pathspecs one git run is given, well short of the smallest limit on
/// a command line, macOS's 1 MiB, as GNU xargs's default of 128 KiB is.
const BATCH: usize = 128 * 1024;

/// lychee, run at the repository's root, so that it reads the repository's lychee config
/// and writes its cache there, whichever directory the task was started in.
#[derive(Debug)]
pub(crate) struct Lychee {
    root: PathBuf,
}

/// The files a commit stages among those lychee reads, NUL-separated, as git lists them.
#[derive(Debug)]
struct Staged(Vec<u8>);

impl Lychee {
    /// Checks the links of the files the commit stages, and returns the code to exit
    /// with. The task reads no arguments.
    pub(crate) fn check_staged() -> ExitCode {
        match Self::stages() {
            Ok(code) | Err(code) => code,
        }
    }

    /// Each stage hands the next what it needs, or ends the run with a code it has
    /// already reported.
    fn stages() -> Result<ExitCode, ExitCode> {
        let lychee = Lychee::at_root()?;
        let inputs = lychee.inputs()?;
        let files = lychee.staged(&inputs)?;
        if files.0.is_empty() {
            return Ok(ExitCode::SUCCESS);
        }
        Ok(lychee.check(&files))
    }

    /// lychee at the root of the repository the task runs in, which the environment a
    /// git hook exports names.
    fn at_root() -> Result<Self, ExitCode> {
        let top = Git::inheriting().run(&["rev-parse", "--show-toplevel"]);
        if !top.success() {
            return Err(top.failure(UNCHECKED));
        }
        let line = top.stdout_bytes();
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        Ok(Lychee {
            root: PathBuf::from(OsStr::from_bytes(line)),
        })
    }

    /// The files `lychee .` reads, relative to the root, so that the repository's lychee
    /// config and .gitignore decide which files count, as they do for a run over the
    /// whole tree: a file named to lychee is checked whatever its extension and whether
    /// or not it is ignored.
    fn inputs(&self) -> Result<Vec<String>, ExitCode> {
        let output = self
            .command()
            .args(["--dump-inputs", "."])
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| {
                Stream::Stderr.line(format_args!("{error}"));
                ExitCode::from(error.shell_status())
            })?;
        if !output.status.success() {
            return Err(failed(output.status));
        }
        // A name that is not UTF-8 matches nothing git stores, and goes unchecked.
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.strip_prefix("./").unwrap_or(line).to_owned())
            .collect())
    }

    /// The staged files among `inputs`. git does the matching, taking
    /// them as literal pathspecs, so a name git stores in another Unicode form than the
    /// one on disk (macOS precomposes) still matches. The pathspecs go to git in batches,
    /// as xargs would pass them, since a repository's files can name more than one
    /// command line holds.
    fn staged(&self, inputs: &[String]) -> Result<Staged, ExitCode> {
        let mut staged = Vec::new();
        let mut rest = inputs;
        // With no pathspec, git would list every staged file, so no batch is empty.
        while !rest.is_empty() {
            let mut size = 0;
            let count = rest
                .iter()
                .take_while(|path| {
                    size += path.len() + 1;
                    size <= BATCH
                })
                .count()
                .max(1);
            let (batch, after) = rest.split_at(count);
            let args: Vec<&str> = [
                "--literal-pathspecs",
                "diff",
                "--cached",
                "--name-only",
                "-z",
                "--diff-filter=d",
                "--",
            ]
            .into_iter()
            .chain(batch.iter().map(String::as_str))
            .collect();
            let run = Git::inheriting().call(&args).dir(&self.root).run();
            if !run.success() {
                return Err(run.failure(UNCHECKED));
            }
            staged.extend_from_slice(run.stdout_bytes());
            rest = after;
        }
        Ok(Staged(staged))
    }

    /// Checks the links of `files`, and returns the code to exit with.
    fn check(&self, files: &Staged) -> ExitCode {
        let status = self
            .command()
            .args(["--cache", "--no-progress", "--files-from", "-"])
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut running| {
                // lychee that stopped before reading its list, as on a config it
                // cannot load, says why in its status.
                let _ = running.feed(&files.list());
                running.wait()
            });
        match status {
            Ok(status) if status.success() => ExitCode::SUCCESS,
            Ok(_) => {
                // The bypass at hand otherwise is --no-verify, which drops every other
                // check with it.
                if let Some(task) = skip_name() {
                    Stream::Stderr.line(format_args!(
                        "If a failing link is not one this commit adds, skip only this check: MISE_TASK_SKIP={task} git commit ..."
                    ));
                }
                ExitCode::FAILURE
            }
            Err(error) => {
                Stream::Stderr.line(format_args!("{error}"));
                ExitCode::from(error.shell_status())
            }
        }
    }

    fn command(&self) -> Program {
        Program::new("lychee").current_dir(&self.root)
    }
}

impl Staged {
    /// The files as the lines lychee reads: lychee expands an input as a
    /// glob and skips a line starting with `#` as a comment, so `[`, `*` and `?` are
    /// bracketed to match only themselves, and each path gets `./`.
    fn list(&self) -> Vec<u8> {
        let mut list = Vec::new();
        for path in self
            .0
            .split(|&byte| byte == 0)
            .filter(|path| !path.is_empty())
        {
            list.extend_from_slice(b"./");
            for &byte in path {
                if matches!(byte, b'*' | b'?' | b'[') {
                    list.extend_from_slice(&[b'[', byte, b']']);
                } else {
                    list.push(byte);
                }
            }
            list.push(b'\n');
        }
        list
    }
}

/// The code to end with where lychee failed: its own status, or `UNCHECKED` where a
/// signal stopped it and it has none.
fn failed(status: ExitStatus) -> ExitCode {
    ExitCode::from(
        status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(UNCHECKED),
    )
}

/// What `MISE_TASK_SKIP` takes to skip the task mise is running, or none where mise is
/// not running one. mise names a file task by its own name, whichever of its names or
/// aliases it was called by, and without the `.sh` the skip needs, so the extension is
/// added; a name that has it already, as an inline task named after the file task has,
/// is taken as it stands. An inline task named without it cannot be told from a file
/// task here, and is given a skip that does not match it.
fn skip_name() -> Option<String> {
    let task = std::env::var("MISE_TASK_NAME")
        .ok()
        .filter(|task| !task.is_empty())?;
    Some(format!("{}.sh", task.strip_suffix(".sh").unwrap_or(&task)))
}
