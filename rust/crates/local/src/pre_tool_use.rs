//! The `PreToolUse` hooks for Bash, one subcommand each, sharing how a hook reads the call
//! Claude is about to make and hands back its verdict.

pub(crate) mod ask_email_in_request;
pub(crate) mod block_bsd_sed_inplace;
pub(crate) mod block_bsd_stat_format;
pub(crate) mod block_piped_git_push;

use std::io::{self, Write};
use std::process::{Command, ExitCode, Stdio};

use serde_json::{Map, Value, json};

/// A Bash call Claude is about to make, as the hook's input on stdin gives it.
pub(crate) struct BashCall {
    command: String,
    /// The call's whole tool input, the command among it.
    tool_input: Map<String, Value>,
}

impl BashCall {
    /// The call on stdin, or `None` where stdin is unreadable or holds no Bash command,
    /// which the hook lets through, as any failure of its own.
    pub(crate) fn read() -> Option<Self> {
        let mut input: Value = serde_json::from_reader(io::stdin().lock()).ok()?;
        let Value::Object(tool_input) = input.get_mut("tool_input")?.take() else {
            return None;
        };
        let command = tool_input.get("command")?.as_str()?.to_owned();
        Some(BashCall {
            command,
            tool_input,
        })
    }
}

/// A tool that comes as GNU's or BSD's, of which a hook blocks a BSD form only where the
/// one on PATH is GNU's.
pub(crate) struct Tool(&'static str);

impl Tool {
    /// Whether the tool on PATH is GNU's, the one of the two that knows `--version`.
    pub(crate) fn is_gnu(&self) -> bool {
        Command::new(self.0)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

/// What a hook decides about a call.
pub(crate) enum Verdict {
    /// The call goes ahead, as far as this hook is concerned.
    Pass,
    /// The call is blocked, and Claude is told why.
    Block(&'static str),
    /// The user is asked whether the call, as `updated_input` gives it, goes ahead, and
    /// Claude is told why.
    Ask {
        reason: &'static str,
        updated_input: Map<String, Value>,
    },
}

impl Verdict {
    /// Ends the hook as Claude Code reads it: exit 2 blocks the call and hands stderr
    /// back to Claude, and exit 0 lets it through, or with a decision on stdout puts it
    /// to the user.
    pub(crate) fn report(self) -> ExitCode {
        match self {
            Verdict::Pass => ExitCode::SUCCESS,
            Verdict::Block(reason) => {
                let _ = io::stderr().write_all(reason.as_bytes());
                ExitCode::from(2)
            }
            Verdict::Ask {
                reason,
                updated_input,
            } => {
                // The decision and the updated input go in one response, since auto mode
                // rejects an updated input that comes without a decision.
                let output = json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "ask",
                        "permissionDecisionReason": reason,
                        "updatedInput": updated_input,
                    }
                });
                let _ = writeln!(io::stdout(), "{output:#}");
                ExitCode::SUCCESS
            }
        }
    }
}
