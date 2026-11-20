//! `block-bsd-sed-inplace`: blocks BSD's `sed -i ''` where the sed on PATH is GNU sed.
//!
//! GNU sed's -i takes no separate suffix argument, so the empty string becomes the
//! script and the real script a file name, or with -e the empty string becomes a file
//! name; either way the call exits 2, and without -e it edits nothing. Where sed is BSD
//! sed the form is the right one, so the hook passes everything there.

use std::process::ExitCode;
use std::sync::LazyLock;

use regex::Regex;

use super::{BashCall, Tool, Verdict};

/// A bare `sed` given -i and then an empty suffix, options before -i included, at the
/// start of any line or after a separator or a space. A path such as /usr/bin/sed names
/// its own sed, and a quote or `|` right before the word marks an argument to another
/// command. The same text after a space inside a quoted argument is still caught.
static EMPTY_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:^|[[:space:];&(])sed +(?:-[a-zA-Z]+ +)*-i +(?:''|"")"#)
        .expect("the pattern compiles")
});

/// What Claude is told when the call is blocked.
const BLOCKED: &str = "\
Blocked: sed on PATH here is GNU sed, whose -i takes no separate suffix
argument, so the empty string after -i is read as the script or as a file
name and the call fails. Write `sed -i 's/.../.../' file`, or use the Edit
tool where the edit is not mechanical.
";

/// Judges the call on stdin.
pub(crate) fn run() -> ExitCode {
    if !Tool("sed").is_gnu() {
        return Verdict::Pass.report();
    }
    match BashCall::read() {
        Some(call) if call.edits_with_empty_suffix() => Verdict::Block(BLOCKED),
        _ => Verdict::Pass,
    }
    .report()
}

impl BashCall {
    /// Whether the command gives a bare sed -i an empty suffix.
    fn edits_with_empty_suffix(&self) -> bool {
        EMPTY_SUFFIX.is_match(&self.command)
    }
}
