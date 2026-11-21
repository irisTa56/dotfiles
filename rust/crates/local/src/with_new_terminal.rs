//! `with-new-terminal`: runs a shell script in a new Terminal window, leaving out its
//! comment and blank lines.
//!
//! A relative script path is read from the directory the command runs in, which the
//! shell function calling it leaves as the one it was called from.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

/// What a run is told on arguments it cannot read.
const USAGE: &str = "\
Usage: with-new-terminal <script.sh> [--cwd <dir>] [--title <title>]

Runs a shell script in a new Terminal window, leaving out its comment and
blank lines.

  --cwd <dir>      the directory the script runs in
  --title <title>  the window's title; the script's file name by default
";

/// A run's arguments: one script path, and the options that may come before or after it.
#[derive(Debug)]
struct Request<'a> {
    script_path: &'a str,
    cwd: Option<&'a str>,
    title: Option<&'a str>,
}

/// Runs the script `args` names.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let Some(request) = Request::parse(args) else {
        return fail(format_args!("{USAGE}"), 1);
    };
    // A byte that is not UTF-8 is replaced rather than refused, so one in a comment
    // line goes with the line.
    let contents = match fs::read(expand_home(request.script_path)) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            return fail(
                format_args!("cannot read {}: {error}", request.script_path),
                1,
            );
        }
    };
    let applescript = request.applescript(&contents);
    // osascript prints the window it opened, which no caller reads.
    match Command::new("osascript")
        .arg("-e")
        .arg(applescript)
        .stdout(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            // A signal ends it with the status a shell gives such a command.
            let code = status
                .code()
                .or_else(|| status.signal().map(|signal| 128 + signal))
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1);
            fail(format_args!("osascript failed (exit {code})"), code)
        }
        Err(error) => fail(format_args!("osascript failed: {error}"), 1),
    }
}

/// Reports `message` and ends the run with `code`.
fn fail(message: fmt::Arguments<'_>, code: u8) -> ExitCode {
    let _ = writeln!(io::stderr(), "{message}");
    ExitCode::from(code)
}

impl<'a> Request<'a> {
    /// What osascript runs to open the window and run `contents`, the script's text.
    fn applescript(&self, contents: &str) -> String {
        let cd = self.cwd.map(|dir| format!("cd {dir}"));
        let lines: Vec<&str> = ["setopt no_bang_hist && clear"]
            .into_iter()
            .chain(cd.as_deref())
            .chain(contents.split('\n').filter(|line| {
                let trimmed = line.trim();
                !(trimmed.is_empty() || trimmed.starts_with('#'))
            }))
            .collect();
        let title = self.title.map_or_else(
            || {
                Path::new(self.script_path)
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
            },
            str::to_owned,
        );
        format!(
            "tell application \"Terminal\"\n  do script \"{}\"\n  set custom title of front window to \"{}\"\nend tell\n",
            escape(lines.join("\n").trim_end()),
            escape(&title)
        )
    }

    /// The request `args` make, or `None` where they make none.
    ///
    /// An option takes its value after `=` or as the next argument, a later one replaces
    /// an earlier one, and `--` ends the options.
    fn parse(args: &'a [String]) -> Option<Self> {
        let mut cwd = None;
        let mut title = None;
        let mut positional = Vec::new();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            if arg == "--" {
                positional.extend(rest.by_ref().map(String::as_str));
                break;
            }
            if !is_switch(arg) {
                positional.push(arg.as_str());
                continue;
            }
            let (name, inline) = match arg.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (arg.as_str(), None),
            };
            let slot = match name {
                "--cwd" => &mut cwd,
                "--title" => &mut title,
                _ => return None,
            };
            let value = match inline {
                Some(value) => value,
                None => match rest.next() {
                    Some(value) if !is_switch(value) => value.as_str(),
                    _ => return None,
                },
            };
            *slot = Some(value);
        }
        match positional.as_slice() {
            [script_path] => Some(Request {
                script_path,
                cwd,
                title,
            }),
            _ => None,
        }
    }
}

/// Whether `arg` reads as an option rather than a value: it opens with `-`, and
/// neither ends there nor goes on with a space or a digit.
fn is_switch(arg: &str) -> bool {
    arg.strip_prefix('-').is_some_and(|rest| {
        !(rest.is_empty() || rest.starts_with(|c: char| c == ' ' || c.is_ascii_digit()))
    })
}

/// `path` with a leading `~` the shell left alone, as in a quoted path, read as the
/// home directory.
fn expand_home(path: &str) -> PathBuf {
    let rest = match path.strip_prefix('~') {
        Some("") => "",
        Some(rest) if rest.starts_with('/') => &rest[1..],
        _ => return PathBuf::from(path),
    };
    match std::env::var_os("HOME") {
        Some(home) => Path::new(&home).join(rest),
        None => PathBuf::from(path),
    }
}

/// `s` as the inside of a string literal of the script osascript runs.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
