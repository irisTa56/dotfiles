//! `ask-email-in-request`: the `PreToolUse` hook that asks the user before a curl or wget
//! call that carries their email address or its local part, run with a fake git on PATH
//! that answers for the global `user.email` alone.

use serde_json::{Value, json};

use crate::pre_tool_use::{Hook, Run, Tool};

/// The hook, asking git for the user's email address.
const HOOK: Hook = Hook::asking("ask-email-in-request", "git");

/// A fake git whose global `user.email` is `$email`, answering nothing else.
macro_rules! git_with_email {
    ($email:literal) => {
        Tool::Fake(concat!(
            "[ \"$*\" = 'config --global user.email' ] && printf '%s\\n' '",
            $email,
            "'"
        ))
    };
}

/// A fake git whose global `user.email` is first.last@example.com.
const GIT: Tool = git_with_email!("first.last@example.com");

/// What Claude is told about why the user is asked.
const REASON: &str = "It carries the user's email address: allow it only if the service requires the address; otherwise -A project-research identifies the caller.";

/// The hook's verdict on a call whose tool input becomes `updated_input`.
fn asked(updated_input: &Value) -> Run {
    Run::asked(&json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": REASON,
            "updatedInput": updated_input,
        }
    }))
}

/// The verdict on a Bash call of `command` described as the runner describes it.
fn asked_about(command: &str) -> Run {
    asked(&json!({
        "command": command,
        "description": "Sends the user's email address: Runs a command",
    }))
}

// An agent puts the address into a User-Agent where a name alone would do, and some
// services do require it, so the user is asked.
#[test]
fn asks_about_a_curl_carrying_the_address() {
    let command = "curl -A 'bot (first.last@example.com)' https://api.example.com";
    assert_eq!(HOOK.run("asks", GIT, command), asked_about(command));
}

// Only a request made by curl or wget is asked about, so the address elsewhere passes.
#[test]
fn passes_the_address_outside_a_request() {
    let command = "git log --author=first.last@example.com";
    assert_eq!(HOOK.run("outside", GIT, command), Run::passed());
}

// A request that carries neither the address nor its local part passes, the domain alone
// included.
#[test]
fn passes_a_request_without_the_address() {
    for command in [
        "curl -A 'bot (project-research)' https://api.example.com",
        "curl -A 'bot (example.com)' https://api.example.com",
    ] {
        assert_eq!(
            (command, HOOK.run("without", GIT, command)),
            (command, Run::passed())
        );
    }
}

// The local part alone is asked about, in any case.
#[test]
fn asks_about_the_local_part_in_any_case() {
    for command in [
        "curl -A 'bot (first.last)' https://api.example.com",
        "curl -A 'bot (First.LAST@Example.com)' https://api.example.com",
    ] {
        assert_eq!(
            (command, HOOK.run("local-part", GIT, command)),
            (command, asked_about(command))
        );
    }
}

// The local part is matched as written, so its dot matches only a dot.
#[test]
fn matches_the_local_part_as_written() {
    let command = "curl -A 'bot (firstXlast)' https://api.example.com";
    assert_eq!(HOOK.run("as-written", GIT, command), Run::passed());
}

// A bare curl or wget is a request at the start of a line or after a space, ;, &, ( or
// |, followed by a space or a tab, and the address may come before it.
#[test]
fn asks_about_each_form_of_request() {
    for command in [
        "wget -U 'bot (first.last)' https://api.example.com",
        "echo first.last; curl https://api.example.com",
        "cd x && curl -A first.last https://api.example.com",
        "cd x;curl -A first.last https://api.example.com",
        "cd x &curl -A first.last https://api.example.com",
        "(curl -A first.last https://api.example.com)",
        "echo x |curl -A first.last https://api.example.com",
        "echo x\tcurl -A first.last https://api.example.com",
        "curl\t-A first.last https://api.example.com",
        "echo x  curl  -A first.last https://api.example.com",
        "echo first.last\ncurl https://api.example.com",
    ] {
        assert_eq!(
            (command, HOOK.run("forms", GIT, command)),
            (command, asked_about(command))
        );
    }
}

// Anything else passes: a path, a quote, a backtick or a word right before the name, a
// name that goes on or ends its line, a name split across lines, and a name beside a
// Unicode space, which the shell does not split words on.
#[test]
fn passes_what_is_no_bare_request() {
    for command in [
        "/usr/bin/curl -A first.last https://api.example.com",
        "'curl' -A first.last https://api.example.com",
        "\"curl\" -A first.last https://api.example.com",
        "`curl -A first.last https://api.example.com`",
        "xcurl -A first.last https://api.example.com",
        "curly -A first.last https://api.example.com",
        "wgetx -U first.last https://api.example.com",
        "echo curl\n-A first.last",
        "echo cur\nl -A first.last",
        "echo x\u{a0}curl -A first.last https://api.example.com",
        "curl\u{3000}-A first.last https://api.example.com",
    ] {
        assert_eq!(
            (command, HOOK.run("no-bare", GIT, command)),
            (command, Run::passed())
        );
    }
}

// The local part runs up to the first @ of the address, and is all of an address with no
// @.
#[test]
fn takes_the_local_part_up_to_the_first_at() {
    for (git, command) in [
        (
            git_with_email!("first@last@example.com"),
            "curl -A 'bot (first)' https://api.example.com",
        ),
        (
            git_with_email!("firstlast"),
            "curl -A 'bot (firstlast)' https://api.example.com",
        ),
    ] {
        assert_eq!(
            (command, HOOK.run("up-to-at", git, command)),
            (command, asked_about(command))
        );
    }
}

// Without git, where git fails, even having printed an address, or with an empty local
// part, there is nothing to look for, and the call passes.
#[test]
fn passes_without_a_local_part() {
    let command = "curl -A 'bot (first.last@example.com)' https://api.example.com";
    for (case, git) in [
        ("no-git", Tool::Missing),
        ("no-global-address", Tool::Fake("exit 1")),
        (
            "git-fails",
            Tool::Fake("printf '%s\\n' first.last@example.com; exit 1"),
        ),
        ("empty-local-part", git_with_email!("@example.com")),
    ] {
        assert_eq!((case, HOOK.run(case, git, command)), (case, Run::passed()));
    }
}

// The cause heads the description, which the permission prompt shows as its heading, and
// stands alone where the call has no description, a null one or an empty one. The rest of the tool
// input goes back as it came.
#[test]
fn heads_the_description_with_the_cause() {
    let command = "curl -A first.last https://api.example.com";
    for (case, description) in [
        ("no-description", None),
        ("null-description", Some(Value::Null)),
        ("empty-description", Some(json!(""))),
    ] {
        let mut tool_input = json!({ "command": command, "timeout": 5000 });
        if let Some(description) = description {
            tool_input["description"] = description;
        }
        let input = json!({ "tool_name": "Bash", "tool_input": tool_input });
        assert_eq!(
            (case, HOOK.run_with_input(case, GIT, &input.to_string())),
            (
                case,
                asked(&json!({
                    "command": command,
                    "timeout": 5000,
                    "description": "Sends the user's email address",
                }))
            )
        );
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
        r#"{"tool_input": {"command": "curl -A first.last x", "description": 5}}"#,
    ] {
        assert_eq!(
            (input, HOOK.run_with_input("own-failure", GIT, input)),
            (input, Run::passed())
        );
    }
}
