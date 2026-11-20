//! `block-piped-git-push`: blocks a `git push` chained with && after a piped git merge,
//! rebase, pull or commit.
//!
//! Without pipefail, && tests the last stage of the pipe, so
//! `git merge --ff-only x | tail -1 && git push` pushes even when the merge failed.

use std::process::ExitCode;
use std::sync::LazyLock;

use regex::Regex;

use super::{BashCall, Verdict};

/// A git merge, rebase, pull or commit piped into a stage that && then follows with a
/// git push.
static PIPED_THEN_PUSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"git[^|;]* (merge|rebase|pull|commit)[^|;]*\|[^;]*&&[^;]*git[^;]* push")
        .expect("the pattern compiles")
});

/// What Claude is told when the call is blocked.
const BLOCKED: &str = "\
Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe
stage and the push would run even if that step failed. Drop the pipe, or push
in a separate command after checking the result.
";

/// Judges the call on stdin.
pub(crate) fn run() -> ExitCode {
    match BashCall::read() {
        Some(call) if call.pushes_after_a_pipe() => Verdict::Block(BLOCKED),
        _ => Verdict::Pass,
    }
    .report()
}

impl BashCall {
    /// Whether a line of the command pushes after a piped git step, chained with &&. A
    /// line that mentions pipefail is taken to set it, and is skipped.
    fn pushes_after_a_pipe(&self) -> bool {
        self.command
            .split('\n')
            .filter(|line| !line.contains("pipefail"))
            .any(|line| PIPED_THEN_PUSH.is_match(line))
    }
}
