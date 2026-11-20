//! What the `PreToolUse` hook tests share: running a hook on a Bash call, with a fake
//! standing in for the tool on PATH the hook asks, where it asks one.

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The tool on PATH: GNU's or BSD's, told apart as the hooks tell them, by whether
/// `--version` succeeds, none, or a fake running the given shell script body.
#[derive(Clone, Copy)]
pub(crate) enum Tool {
    Gnu,
    Bsd,
    Missing,
    Fake(&'static str),
}

/// What a run left: its exit status, its stdout and its stderr.
#[derive(Debug, PartialEq)]
pub(crate) struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

impl Run {
    pub(crate) fn passed() -> Self {
        Run {
            code: Some(0),
            out: String::new(),
            err: String::new(),
        }
    }

    /// What a hook leaves when it asks the user about a call, sending `output` as its
    /// verdict.
    pub(crate) fn asked(output: &serde_json::Value) -> Self {
        Run {
            code: Some(0),
            out: format!(
                "{}\n",
                serde_json::to_string_pretty(output).expect("the verdict serializes")
            ),
            err: String::new(),
        }
    }
}

/// A hook, run as its `dotfiles-local` subcommand.
pub(crate) struct Hook {
    /// The subcommand.
    name: &'static str,
    /// The tool on PATH the hook asks, if it asks any.
    tool: Option<&'static str>,
    /// What Claude is told when the hook blocks a call, if it blocks any.
    blocked: Option<&'static str>,
}

impl Hook {
    /// The hook run as the subcommand `name`, judging by the kind of `tool` on PATH and
    /// telling Claude `blocked` when it blocks a call.
    pub(crate) const fn new(name: &'static str, tool: &'static str, blocked: &'static str) -> Self {
        Hook {
            name,
            tool: Some(tool),
            blocked: Some(blocked),
        }
    }

    /// The hook run as the subcommand `name`, asking the user rather than blocking, and
    /// asking `tool` on PATH what it judges by.
    pub(crate) const fn asking(name: &'static str, tool: &'static str) -> Self {
        Hook {
            name,
            tool: Some(tool),
            blocked: None,
        }
    }

    /// The hook run as the subcommand `name`, judging by the command alone and telling
    /// Claude `blocked` when it blocks a call. It is run under `Tool::Missing`, which
    /// leaves PATH empty.
    pub(crate) const fn judging_by_the_command(name: &'static str, blocked: &'static str) -> Self {
        Hook {
            name,
            tool: None,
            blocked: Some(blocked),
        }
    }

    /// What the hook leaves when it blocks a call.
    pub(crate) fn blocked(&self) -> Run {
        Run {
            code: Some(2),
            out: String::new(),
            err: self
                .blocked
                .expect("the hook blocks calls rather than asking")
                .to_owned(),
        }
    }

    /// Runs the hook on a Bash call of `command`, under the tool `tool`.
    pub(crate) fn run(&self, test: &str, tool: Tool, command: &str) -> Run {
        let input = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": command, "description": "Runs a command" },
        });
        self.run_with_input(test, tool, &input.to_string())
    }

    /// Runs the hook with `input` as its stdin, under the tool `tool`.
    pub(crate) fn run_with_input(&self, test: &str, tool: Tool, input: &str) -> Run {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg(self.name)
            .env("PATH", self.bin(test, tool))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("dotfiles-local starts");
        // The hook need not read its input, as where the tool is the BSD one.
        let _ = child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input.as_bytes());
        let output = child.wait_with_output().expect("dotfiles-local ends");
        Run {
            code: output.status.code(),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// A directory of the test's own whose `bin`, alone on PATH, holds the fake tool.
    fn bin(&self, test: &str, tool: Tool) -> PathBuf {
        let bin = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(self.name)
            .join(test)
            .join("bin");
        let _ = fs::remove_dir_all(&bin);
        fs::create_dir_all(&bin).expect("the sandbox is created");
        let body = match tool {
            Tool::Gnu => "exit 0",
            Tool::Bsd => "exit 1",
            Tool::Fake(body) => body,
            Tool::Missing => return bin,
        };
        let path = bin.join(
            self.tool
                .expect("a hook judging by the command alone runs under Tool::Missing"),
        );
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
        bin
    }
}
