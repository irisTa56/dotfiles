//! The `PreToolUse` hook for Bash that runs as a subcommand, and how it reads the call
//! Claude is about to make and hands back its verdict.

pub(crate) mod ask_email_in_request;

use serde_json::{Map, Value, json};
use std::io::{self, Write};

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

/// What a hook decides about a call.
pub(crate) enum Verdict {
    /// The call goes ahead, as far as this hook is concerned.
    Pass,
    /// The user is asked whether the call, as `updated_input` gives it, goes ahead, and
    /// Claude is told why.
    Ask {
        reason: &'static str,
        updated_input: Map<String, Value>,
    },
}

impl Verdict {
    /// Ends the hook as Claude Code reads it: exit 0 lets the call through, or with a
    /// decision on stdout puts it to the user. A decision that cannot be written lets the
    /// call through, as any failure of the hook's own.
    pub(crate) fn report(self) {
        match self {
            Verdict::Pass => {}
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
            }
        }
    }
}
