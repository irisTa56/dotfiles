//! How each end of a run but a pass is reported: the exit code, and stdout and stderr,
//! each whole. git is a stand-in where the end is one the real one cannot be made to
//! reach on cue.

use std::ffi::OsStr;

use super::{LIMIT, over, skill};
use crate::support::checkout::Checkout;
use crate::support::invocation::Invocation;
use crate::support::stand_in::{FAILING, KILLED, StandIns};

/// A run of the task in a repository tracking a short skill, finding git on `path`.
fn run_on(path: impl AsRef<OsStr>) -> (Checkout, Invocation) {
    let checkout = Checkout::new(&[(".claude/skills/a/SKILL.md", &skill("short\n"))]);
    let run = checkout
        .dotfiles_tasks()
        .arg("skills-size-check")
        .env("PATH", path);
    (checkout, run)
}

#[test]
fn says_git_could_not_run_and_exits_2_where_git_is_not_on_path() {
    let bin = StandIns::new();
    let (_checkout, run) = run_on(bin.dir());
    run.run_apart().expect(
        2,
        "",
        "could not run git: No such file or directory (os error 2)\n",
    );
}

#[test]
fn passes_on_what_git_said_and_its_status_where_git_cannot_list_the_skills() {
    let bin = StandIns::new().git_on("ls-files", FAILING);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(5, "", "stand-in: failed\n");
}

#[test]
fn exits_2_where_a_signal_stops_git_listing_the_skills() {
    let bin = StandIns::new().git_on("ls-files", KILLED);
    let (_checkout, run) = run_on(bin.path());
    run.run_apart().expect(2, "", "");
}

#[test]
fn says_git_tracks_no_skill_and_exits_1_where_it_lists_none() {
    Checkout::new(&[])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run_apart()
        .expect(
            1,
            "",
            "git tracks no skill at :(glob).claude/skills/*/SKILL.md to check.\n",
        );
}

// The line of each skill that fails goes out as it is measured, and the skills after it
// are measured all the same.
#[test]
fn names_each_skill_past_the_limit_and_exits_1() {
    Checkout::new(&[
        (".claude/skills/a/SKILL.md", &skill(&over())),
        (".claude/skills/b/SKILL.md", &skill("short\n")),
        (".claude/skills/c/SKILL.md", &skill(&over())),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .run_apart()
    .expect(
        1,
        "",
        &[
            past(".claude/skills/a/SKILL.md"),
            past(".claude/skills/c/SKILL.md"),
        ]
        .concat(),
    );
}

/// The line of a skill at `path` whose body is one code unit past the limit.
fn past(path: &str) -> String {
    format!(
        "{path}: {} UTF-16 code units, past the {LIMIT} a compaction re-attaches; shorten it, or move into references/ only what it reads at one moment, keeping in the body what a step needs before the body points there\n",
        LIMIT + 1
    )
}
