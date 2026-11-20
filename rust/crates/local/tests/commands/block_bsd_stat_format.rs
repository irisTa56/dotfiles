//! `block-bsd-stat-format`: the `PreToolUse` hook that blocks BSD's `stat -f <format>`
//! where the stat on PATH is GNU stat, run with a fake stat standing in for either kind.

use crate::pre_tool_use::{Hook, Run, Tool};

/// The hook, judging by the stat on PATH.
const HOOK: Hook = Hook::new(
    "block-bsd-stat-format",
    "stat",
    "\
Blocked: stat on PATH here is GNU stat, whose -f reports the file system and
takes no format, so the format is read as a file name and the call fails.
Write `stat -c '%Y %s %n' file` for mtime, size and name, or
`stat -c '%y' file` for a readable mtime; on macOS, /usr/bin/stat is BSD stat.
Where the command only quotes the form in text, such as a commit message,
reword that text.
",
);

// GNU stat reads the format as a file name, so the call is blocked.
#[test]
fn blocks_a_format_under_gnu_stat() {
    assert_eq!(
        HOOK.run("gnu", Tool::Gnu, "stat -f %z file"),
        HOOK.blocked()
    );
}

// BSD stat takes the format after -f, the form it documents, so the call passes.
#[test]
fn passes_a_format_under_bsd_stat() {
    assert_eq!(HOOK.run("bsd", Tool::Bsd, "stat -f %z file"), Run::passed());
}

// GNU stat's own form, the format after -c, passes.
#[test]
fn passes_gnu_stat_own_form() {
    assert_eq!(
        HOOK.run("gnu-form", Tool::Gnu, "stat -c %s file"),
        Run::passed()
    );
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
            (input, HOOK.run_with_input("own-failure", Tool::Gnu, input)),
            (input, Run::passed())
        );
    }
    assert_eq!(
        HOOK.run("no-stat", Tool::Missing, "stat -f %z file"),
        Run::passed()
    );
}

// Where the shell would run a bare stat with -f and a format, the call is blocked: the
// format quoted or not, attached to -f or not, or with text before its first %; at the
// start of any line, after a separator or a space, and with options before -f. The same
// text after a space inside a quoted argument is caught too.
#[test]
fn blocks_every_place_a_bare_stat_runs() {
    for command in [
        "stat -f '%Sm %N' file",
        r#"stat -f "%z" file"#,
        "stat -f%z file",
        "stat -f'%z' file",
        "stat -f 'size=%z' file",
        "stat -L -f %z file",
        "stat -L  -x  -f  %z file",
        "cd dir;stat -f %z file",
        "true&&stat -f %z file",
        "(stat -f %z file)",
        "echo start\nstat -f %z file",
        "xargs\tstat -f %z",
        "git commit -m 'run stat -f %z here'",
    ] {
        assert_eq!(
            (command, HOOK.run("gnu-blocks", Tool::Gnu, command)),
            (command, HOOK.blocked())
        );
    }
}

// Where the word after -f is a path or an option rather than a format, or the stat is not
// a bare one, the call passes: GNU stat's own -f takes a path, and its format follows -c.
#[test]
fn passes_what_is_no_bare_stat_given_a_format() {
    for command in [
        "stat -f /",
        "stat -f -c%T /",
        "stat -f -c %T /",
        "stat -fc%T /",
        "stat -f",
        "/usr/bin/stat -f %z file",
        "gstat -f %z file",
        "echo x |stat -f %z file",
        "git commit -m 'stat -f %z is BSD'",
        "",
    ] {
        assert_eq!(
            (command, HOOK.run("gnu-passes", Tool::Gnu, command)),
            (command, Run::passed())
        );
    }
}
