//! `block-piped-git-push`: the `PreToolUse` hook that blocks a `git push` chained with &&
//! after a piped git merge, rebase, pull or commit, run with nothing on PATH.

use crate::pre_tool_use::{Hook, Run, Tool};

/// The hook, judging by the command alone.
const HOOK: Hook = Hook::judging_by_the_command(
    "block-piped-git-push",
    "\
Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe
stage and the push would run even if that step failed. Drop the pipe, or push
in a separate command after checking the result.
",
);

/// Runs the hook on a Bash call of `command`. PATH is left empty, as the hook needs no
/// tool.
fn run(test: &str, command: &str) -> Run {
    HOOK.run(test, Tool::Missing, command)
}

// && tests only the last stage of the pipe, so the push would follow a failed merge.
#[test]
fn blocks_a_push_after_a_piped_merge() {
    assert_eq!(
        run("blocks", "git merge --ff-only x | tail -1 && git push"),
        HOOK.blocked()
    );
}

// Each of the four git steps, piped and followed by && and a push, is blocked: with words
// or none between git and the step, between the step and the pipe, between the pipe and
// &&, between && and git, and between git and push, with several spaces in each gap, and
// wherever the chain sits on the line, inside a quoted argument too, where the gap after
// the pipe can be empty.
#[test]
fn blocks_a_push_after_each_piped_step() {
    for command in [
        "git rebase main | tail -1 && git push",
        "git pull | tail -1 && git push",
        "git commit -m x | tail -1 && git push",
        "git -C repo merge x | tail -1 && git push",
        "git pull| tail -1 && git push",
        "git merge x | tail -1 &&git push",
        "git merge x | tail -1 && cd .. && git push",
        "git merge x | tail -1 && git -C repo push origin main",
        "git  merge  x  |  tail -1  &&  git  push",
        "cd repo && git merge x | tail -1 && git push",
        "echo 'git merge x | tail -1 && git push'",
        "echo 'git pull|&&git push'",
    ] {
        assert_eq!(
            (command, run("blocks-each", command)),
            (command, HOOK.blocked())
        );
    }
}

// Where no piped git step is followed by && and a push on the same side of a ;, the call
// passes. A step or a push counts only after a space, so not inside a word such as
// fix/merge or git-push, and a single & runs the push whatever the pipe gave.
#[test]
fn passes_what_is_no_push_after_a_piped_step() {
    for command in [
        "git merge x && git push",
        "git merge x | tail -1; git push",
        "git merge x | tail -1 || git push",
        "git merge x | tail -1 & git push",
        "git checkout fix/merge | cat && git push",
        "git merge x | tail -1 && echo git-push",
        "git merge x | tail -1 && git status",
        "git log | head -1 && git push",
        "git merge x; echo y | tail -1 && git push",
        "git merge x | tail -1 && git fetch; git push",
        "",
    ] {
        assert_eq!((command, run("passes", command)), (command, Run::passed()));
    }
}

// A failure of the hook's own lets the call through with exit 0 and no verdict: exit 2
// would block every Bash call, and any other status would put a hook error in front of
// each.
#[test]
fn passes_on_its_own_failure() {
    for input in [
        "not json",
        "{}",
        r#"{"tool_input": null}"#,
        r#"{"tool_input": {"command": 5}}"#,
    ] {
        assert_eq!(
            (
                input,
                HOOK.run_with_input("own-failure", Tool::Missing, input)
            ),
            (input, Run::passed())
        );
    }
}

// A line that mentions pipefail is taken to set it, under which && tests every stage, so
// it passes. Another line of the same command is still judged.
#[test]
fn skips_a_line_that_mentions_pipefail() {
    assert_eq!(
        run(
            "pipefail",
            "set -o pipefail; git merge x | tail -1 && git push"
        ),
        Run::passed()
    );
    assert_eq!(
        run(
            "pipefail-other-line",
            "set -o pipefail\ngit merge x | tail -1 && git push"
        ),
        HOOK.blocked()
    );
}

// The command is judged a line at a time, so a chain split across lines passes, as do
// pieces of one on separate lines.
#[test]
fn judges_each_line_on_its_own() {
    for command in [
        "git merge x | tail -1 &&\ngit push",
        "git merge x\necho y | tail -1 && git push",
    ] {
        assert_eq!((command, run("lines", command)), (command, Run::passed()));
    }
}
