//! `with-new-terminal`: runs a shell script in a new Terminal window, leaving out its
//! comment and blank lines.
//!
//! A relative script path is read from the directory the command runs in, which the
//! shell function calling it leaves as the one it was called from.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Args;

use crate::failure::{Failure, exit_code};

/// Run a shell script in a new Terminal window, leaving out its comment and blank lines.
///
/// Options may come before or after the script, and a later one replaces an earlier one.
#[derive(Debug, Args)]
#[command(args_override_self = true)]
pub(crate) struct Request {
    /// The script to run
    #[arg(value_name = "SCRIPT.SH")]
    script_path: String,
    /// The directory the script runs in
    #[arg(long, value_name = "DIR", allow_hyphen_values = true, value_parser = value)]
    cwd: Option<String>,
    /// The window's title [default: the script's file name]
    #[arg(long, allow_hyphen_values = true, value_parser = value)]
    title: Option<String>,
}

impl Request {
    /// Runs the script in a new window.
    pub(crate) fn run(self) -> Result<(), Failure> {
        // A byte that is not UTF-8 is replaced rather than refused, so one in a comment
        // line goes with the line.
        let contents = fs::read(expand_home(&self.script_path))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|error| {
                Failure::new(format_args!("cannot read {}: {error}", self.script_path), 1)
            })?;
        let applescript = self.applescript(&contents);
        // osascript prints the window it opened, which no caller reads.
        let status = Command::new("osascript")
            .arg("-e")
            .arg(applescript)
            .stdout(Stdio::null())
            .status()
            .map_err(|error| Failure::new(format_args!("osascript failed: {error}"), 1))?;
        if status.success() {
            return Ok(());
        }
        // A signal ends it with the status a shell gives such a command.
        let code = exit_code(status);
        Err(Failure::new(
            format_args!("osascript failed (exit {code})"),
            code,
        ))
    }

    /// What osascript runs to open the window and run `contents`, the script's text.
    fn applescript(&self, contents: &str) -> String {
        let cd = self.cwd.as_deref().map(|dir| format!("cd {dir}"));
        let lines: Vec<&str> = ["setopt no_bang_hist && clear"]
            .into_iter()
            .chain(cd.as_deref())
            .chain(contents.split('\n').filter(|line| {
                let trimmed = line.trim();
                !(trimmed.is_empty() || trimmed.starts_with('#'))
            }))
            .collect();
        let title = self.title.as_deref().map_or_else(
            || {
                Path::new(&self.script_path)
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
}

/// An option's value, which may open with `-` only where neither the end nor a space nor
/// a digit follows, since one that reads as an option is likelier an option written
/// where its value was left out. clap checks a value given after `=` the same way.
fn value(arg: &str) -> Result<String, &'static str> {
    let reads_as_an_option = arg.strip_prefix('-').is_some_and(|rest| {
        !(rest.is_empty() || rest.starts_with(|c: char| c == ' ' || c.is_ascii_digit()))
    });
    if reads_as_an_option {
        return Err("it reads as an option rather than a value");
    }
    Ok(arg.to_owned())
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
