//! `block-bsd-sed-inplace`: the `PreToolUse` hook that blocks BSD's `sed -i ''` where the
//! sed on PATH is GNU sed, run with a fake sed standing in for either kind.

use crate::pre_tool_use::{Hook, Run, Tool};

/// The hook, judging by the sed on PATH.
const HOOK: Hook = Hook::new(
    "block-bsd-sed-inplace",
    "sed",
    "\
Blocked: sed on PATH here is GNU sed, whose -i takes no separate suffix
argument, so the empty string after -i is read as the script or as a file
name and the call fails. Write `sed -i 's/.../.../' file`, or use the Edit
tool where the edit is not mechanical.
",
);

// GNU sed reads the empty suffix as the script or a file name, so the call is blocked.
#[test]
fn blocks_an_empty_suffix_under_gnu_sed() {
    assert_eq!(
        HOOK.run("gnu", Tool::Gnu, "sed -i '' 's/a/b/' file"),
        HOOK.blocked()
    );
}

// BSD sed takes the empty string as the suffix, the form it documents, so the call passes.
#[test]
fn passes_an_empty_suffix_under_bsd_sed() {
    assert_eq!(
        HOOK.run("bsd", Tool::Bsd, "sed -i '' 's/a/b/' file"),
        Run::passed()
    );
}

// GNU sed's own form, with no separate suffix, passes.
#[test]
fn passes_gnu_sed_own_form() {
    assert_eq!(
        HOOK.run("gnu-form", Tool::Gnu, "sed -i 's/a/b/' file"),
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
        HOOK.run("no-sed", Tool::Missing, "sed -i '' 's/a/b/' file"),
        Run::passed()
    );
}

// Where the shell would run a bare sed with -i and an empty suffix, the call is blocked:
// at the start of any line, after a separator or a space, with options before -i, and
// with either quote. The same text after a space inside a quoted argument is caught too.
#[test]
fn blocks_every_place_a_bare_sed_runs() {
    for command in [
        r#"sed -i "" 's/a/b/' file"#,
        "sed -E -i '' 's/a/b/' file",
        "sed  -n -E  -i  '' 's/a/b/' file",
        "cd dir;sed -i '' 's/a/b/' file",
        "true&&sed -i '' 's/a/b/' file",
        "(sed -i '' 's/a/b/' file)",
        "echo start\nsed -i '' 's/a/b/' file",
        "xargs\tsed -i '' 's/a/b/'",
        "git commit -m 'run sed -i '' here'",
    ] {
        assert_eq!(
            (command, HOOK.run("gnu-blocks", Tool::Gnu, command)),
            (command, HOOK.blocked())
        );
    }
}

// Where the empty string is no separate suffix, or the sed is not a bare one, the call
// passes.
#[test]
fn passes_what_is_no_bare_sed_given_an_empty_suffix() {
    for command in [
        "sed -i.bak 's/a/b/' file",
        "sed -i'' 's/a/b/' file",
        "sed -e '' -i file",
        "/usr/bin/sed -i '' 's/a/b/' file",
        "gsed -i '' 's/a/b/' file",
        "echo x |sed -i '' file",
        "git commit -m 'sed -i '' is BSD'",
        "sed --in-place '' file",
        "",
    ] {
        assert_eq!(
            (command, HOOK.run("gnu-passes", Tool::Gnu, command)),
            (command, Run::passed())
        );
    }
}
