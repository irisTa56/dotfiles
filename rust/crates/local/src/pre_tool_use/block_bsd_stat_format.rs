//! `block-bsd-stat-format`: blocks BSD's `stat -f <format>` where the stat on PATH is
//! GNU stat.
//!
//! GNU stat's -f takes no argument and reports the file system instead of the file, so
//! the format is read as a file name: the call exits 1, and stdout carries the file
//! system's details mixed in with any other output. Written without a space, as in -f%z,
//! the format is an invalid option. Where stat is BSD stat the form is the right one, so
//! the hook passes everything there.

use std::process::ExitCode;
use std::sync::LazyLock;

use regex::Regex;

use super::{BashCall, Tool, Verdict};

/// A bare `stat` given -f and then a format, options before -f included, at the start of
/// any line or after a separator or a space. A format is told from a path by a % in the
/// word after -f, and from an option by not starting with -; attached to -f, only a word
/// starting with % is one. So GNU's own `stat -f <path>`, `stat -f -c%T` and `stat -fc%T`
/// pass. A path such as /usr/bin/stat names its own stat, and a quote or `|` right before
/// the word marks an argument to another command. The same text after a space inside a
/// quoted argument, such as a commit message, is still caught.
static FORMAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:^|[[:space:];&(])stat +(?:-[a-zA-Z]+ +)*-f(?:['"]?%| +['"]?(?:[^-[:space:]'"][^[:space:]'"]*)?%)"#,
    )
    .expect("the pattern compiles")
});

/// What Claude is told when the call is blocked.
const BLOCKED: &str = "\
Blocked: stat on PATH here is GNU stat, whose -f reports the file system and
takes no format, so the format is read as a file name and the call fails.
Write `stat -c '%Y %s %n' file` for mtime, size and name, or
`stat -c '%y' file` for a readable mtime; on macOS, /usr/bin/stat is BSD stat.
Where the command only quotes the form in text, such as a commit message,
reword that text.
";

/// Judges the call on stdin.
pub(crate) fn run() -> ExitCode {
    if !Tool("stat").is_gnu() {
        return Verdict::Pass.report();
    }
    match BashCall::read() {
        Some(call) if call.formats_with_f() => Verdict::Block(BLOCKED),
        _ => Verdict::Pass,
    }
    .report()
}

impl BashCall {
    /// Whether the command gives a bare stat -f a format.
    fn formats_with_f(&self) -> bool {
        FORMAT.is_match(&self.command)
    }
}
