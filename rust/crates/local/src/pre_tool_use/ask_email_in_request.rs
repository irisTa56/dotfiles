//! `ask-email-in-request`: asks the user before a curl or wget call that carries their
//! email address or its local part.
//!
//! Agents put the address into a User-Agent for APIs that ask callers to identify
//! themselves, where a name alone would do. The hook asks rather than blocks because some
//! services do require the address, and the command does not say which kind this is.
//!
//! Only a bare `curl` or `wget` with the address written out is matched, which covers the
//! forms seen in use. A path such as /usr/bin/curl, a quote or a backtick right before
//! the word, a request made from another program, and an address assembled from pieces
//! all pass unasked.

use std::process::{Command, ExitCode, Stdio};
use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use serde_json::Value;

use super::{BashCall, Verdict};

/// A bare curl or wget, at the start of a line or after a space, ;, &, ( or |, and
/// followed by a space. A space is an ASCII one, the only kind the shell splits words on.
static REQUEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|[[:space:];&(|])(curl|wget)[[:space:]]").expect("the pattern compiles")
});

/// What heads the call's description, which the permission prompt shows as its heading.
/// The reason below it is drawn as plain text, no more prominent than the description,
/// and an approver can read past it.
const CAUSE: &str = "Sends the user's email address";

/// What Claude is told about why the user is asked.
const REASON: &str = "It carries the user's email address: allow it only if the service requires the address; otherwise -A project-research identifies the caller.";

/// Judges the call on stdin.
pub(crate) fn run() -> ExitCode {
    let Some(local_part) = LocalPart::configured() else {
        return Verdict::Pass.report();
    };
    match BashCall::read() {
        Some(call) if call.makes_a_request() && local_part.is_in(&call.command) => call.ask(),
        _ => Verdict::Pass,
    }
    .report()
}

/// The local part of the user's email address, matched as written, in any case.
struct LocalPart(Regex);

impl LocalPart {
    /// The local part of git's global `user.email`, up to its first @, or `None` where
    /// git has none or the part is empty, which the hook lets through, as any failure of
    /// its own.
    fn configured() -> Option<Self> {
        let output = Command::new("git")
            .args(["config", "--global", "user.email"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let email = String::from_utf8(output.stdout).ok()?;
        let local_part = email.trim_end_matches('\n').split('@').next()?;
        if local_part.is_empty() {
            return None;
        }
        RegexBuilder::new(&regex::escape(local_part))
            .case_insensitive(true)
            .build()
            .ok()
            .map(LocalPart)
    }

    /// Whether `command` holds the local part.
    fn is_in(&self, command: &str) -> bool {
        self.0.is_match(command)
    }
}

impl BashCall {
    /// Whether a line of the command makes a request with curl or wget.
    fn makes_a_request(&self) -> bool {
        self.command.split('\n').any(|line| REQUEST.is_match(line))
    }

    /// Asks the user about the call, its description headed by the cause, which stands
    /// alone where the description is missing, null, false or empty, as jq's `//` has it.
    /// Any other description that is not a string lets the call through, as any failure of
    /// the hook's own.
    fn ask(self) -> Verdict {
        let description = match self.tool_input.get("description") {
            None | Some(Value::Null | Value::Bool(false)) => CAUSE.to_owned(),
            Some(Value::String(description)) if description.is_empty() => CAUSE.to_owned(),
            Some(Value::String(description)) => format!("{CAUSE}: {description}"),
            Some(_) => return Verdict::Pass,
        };
        let mut updated_input = self.tool_input;
        updated_input.insert("description".to_owned(), Value::String(description));
        Verdict::Ask {
            reason: REASON,
            updated_input,
        }
    }
}
