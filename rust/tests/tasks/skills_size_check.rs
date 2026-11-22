//! `dotfiles-tasks skills-size-check`: whether the body of each skill git tracks fits in
//! what a compaction re-attaches, and the wrapper `tasks/skills/size-check.sh`.

mod failures;
mod wrapper;

use tempfile::TempDir;

use crate::support::checkout::Checkout;
use crate::support::invocation::Stream;

/// The longest body that passes, in UTF-16 code units.
const LIMIT: usize = 19_800;

/// A `SKILL.md` with a frontmatter, followed by `body`.
fn skill(body: &str) -> String {
    format!("---\nname: x\ndescription: y\n---\n{body}")
}

/// A `SKILL.md` whose frontmatter has CRLF line ends, followed by `body`.
fn crlf(body: &str) -> String {
    format!("---\r\nname: x\r\ndescription: y\r\n---\r\n{body}")
}

/// A body one code unit past the limit.
fn over() -> String {
    "a".repeat(LIMIT + 1)
}

#[test]
fn reports_a_skill_past_the_limit_with_what_may_move_into_references_and_what_stays() {
    Checkout::new(&[(".claude/skills/big/SKILL.md", &skill(&over()))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(
            1,
            &[
                &format!(".claude/skills/big/SKILL.md: {}", LIMIT + 1),
                "UTF-16 code units",
                &LIMIT.to_string(),
                "references/",
                "before the body points there",
            ],
        );
}

// JavaScript counts a character outside the BMP as two code units, and so does the cut.
#[test]
fn counts_a_character_outside_the_bmp_as_two_code_units() {
    let body = format!("{}😀", "a".repeat(LIMIT - 1));
    Checkout::new(&[(".claude/skills/e/SKILL.md", &skill(&body))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(1, &[&format!(".claude/skills/e/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn passes_a_body_at_the_limit_printing_nothing() {
    let run = Checkout::new(&[(".claude/skills/full/SKILL.md", &skill(&"a".repeat(LIMIT)))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A session sizing a skill against the limit needs the length of one that passes.
#[test]
fn with_verbose_reports_a_passing_skills_length_against_the_limit() {
    let run = Checkout::new(&[(".claude/skills/full/SKILL.md", &skill(&"a".repeat(LIMIT)))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .arg("--verbose")
        .run();
    assert_eq!(
        (run.code(), run.output()),
        (
            0,
            ".claude/skills/full/SKILL.md: 19800 of 19800 UTF-16 code units\n"
        )
    );
}

#[test]
fn with_verbose_and_a_pathspec_reports_the_passing_skills_and_still_fails() {
    Checkout::new(&[
        ("skills/big/SKILL.md", &skill(&over())),
        ("skills/small/SKILL.md", &skill("a")),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .args(["--verbose", ":(glob)skills/*/SKILL.md"])
    .run()
    .expect(
        1,
        &[
            &format!("skills/big/SKILL.md: {}", LIMIT + 1),
            &format!("skills/small/SKILL.md: 1 of {LIMIT}"),
        ],
    );
}

// An option goes before the pathspec, so one written after it is a usage error rather
// than taking effect.
#[test]
fn fails_as_a_usage_error_naming_an_option_after_the_pathspec() {
    Checkout::new(&[(".claude/skills/small/SKILL.md", &skill("a"))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .args([":(glob).claude/skills/*/SKILL.md", "--verbose"])
        .run()
        .expect(2, &["'--verbose'"]);
}

#[test]
fn fails_as_a_usage_error_naming_a_second_pathspec() {
    Checkout::new(&[(".claude/skills/small/SKILL.md", &skill("a"))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .args([":(glob).claude/skills/*/SKILL.md", "skills/*/SKILL.md"])
        .run()
        .expect(2, &["'skills/*/SKILL.md'"]);
}

// Run where git lists nothing, the check would otherwise pass having read no skill.
#[test]
fn fails_when_git_tracks_no_skill_to_check() {
    Checkout::new(&[])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(1, &["git tracks no skill"]);
}

#[test]
fn fails_with_gits_status_where_git_cannot_list_the_skills() {
    let checkout = Checkout::new(&[]);
    checkout.remove(".git");
    checkout
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(128, &["not a git repository"]);
}

// Vendored skills are restored from upstream on every install, so their length is not
// the repository's to keep; a directory with no SKILL.md is no skill.
#[test]
fn leaves_out_an_ignored_skill_and_a_tracked_directory_with_no_skill_md() {
    let run = Checkout::new(&[
        (
            ".gitignore",
            "/.claude/skills/*\n!/.claude/skills/own/\n!/.claude/skills/plugin/\n",
        ),
        (".claude/skills/own/SKILL.md", &skill("a")),
        (".claude/skills/vendored/SKILL.md", &skill(&over())),
        (".claude/skills/plugin/hooks.json", "{}"),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn checks_only_the_skill_md_one_level_under_claude_skills() {
    let run = Checkout::new(&[
        (".claude/skills/x/SKILL.md", &skill("a")),
        (".claude/skills/x/fixtures/y/SKILL.md", &skill(&over())),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn checks_the_skills_a_pathspec_argument_names_in_place_of_claude_skills() {
    let run = Checkout::new(&[
        ("skills/big/SKILL.md", &skill(&over())),
        (".claude/skills/small/SKILL.md", &skill(&over())),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .arg(":(glob)skills/*/SKILL.md")
    .run();
    run.expect(1, &[&format!("skills/big/SKILL.md: {}", LIMIT + 1)]);
    assert!(
        !run.output().contains(".claude/skills/small"),
        "{}",
        run.output()
    );
}

#[test]
fn leaves_a_crlf_frontmatter_out_of_the_count() {
    let run = Checkout::new(&[(".claude/skills/crlf/SKILL.md", &crlf(&"a".repeat(LIMIT)))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn reports_a_skill_past_the_limit_behind_a_crlf_frontmatter() {
    Checkout::new(&[(".claude/skills/crlf/SKILL.md", &crlf(&over()))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(
            1,
            &[&format!(".claude/skills/crlf/SKILL.md: {}", LIMIT + 1)],
        );
}

// With no frontmatter, the whole file is the body: a `---` rule in it is no fence.
#[test]
fn counts_the_whole_of_a_skill_with_no_frontmatter() {
    Checkout::new(&[(".claude/skills/bare/SKILL.md", &over())])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(
            1,
            &[&format!(".claude/skills/bare/SKILL.md: {}", LIMIT + 1)],
        );
}

#[test]
fn counts_the_whole_of_a_skill_with_no_frontmatter_and_a_rule_line_in_it() {
    let contents = format!("{}\n---\nshort tail\n", over());
    Checkout::new(&[(".claude/skills/rule/SKILL.md", &contents)])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(
            1,
            &[&format!(".claude/skills/rule/SKILL.md: {}", LIMIT + 1 + 16)],
        );
}

#[test]
fn fails_on_a_frontmatter_that_never_closes_rather_than_reading_it_as_all_frontmatter() {
    let contents = format!("---\nname: x\n{}", over());
    Checkout::new(&[(".claude/skills/open/SKILL.md", &contents)])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(
            1,
            &[".claude/skills/open/SKILL.md: its frontmatter has no closing ---"],
        );
}

#[test]
fn reads_a_skill_under_a_non_ascii_directory_and_the_ones_after_it() {
    Checkout::new(&[
        (".claude/skills/é/SKILL.md", &skill("a")),
        (".claude/skills/zz/SKILL.md", &skill(&over())),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .run()
    .expect(1, &[&format!(".claude/skills/zz/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn reports_a_skill_past_the_limit_under_a_non_ascii_directory() {
    Checkout::new(&[(".claude/skills/é/SKILL.md", &skill(&over()))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(1, &[&format!(".claude/skills/é/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn passes_a_short_skill_under_a_non_ascii_directory() {
    let run = Checkout::new(&[(".claude/skills/é/SKILL.md", &skill("a"))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// The check reads the working tree, so a tracked skill whose file is gone from disk, the
// deletion unstaged, must not end the run before the others are read.
#[test]
fn passes_over_a_tracked_skill_deleted_from_disk_and_reads_the_rest() {
    let checkout = Checkout::new(&[
        (".claude/skills/aa/SKILL.md", &skill("a")),
        (".claude/skills/zz/SKILL.md", &skill(&over())),
    ]);
    checkout.remove(".claude/skills/aa/SKILL.md");
    let run = checkout.dotfiles_tasks().arg("skills-size-check").run();
    run.expect(1, &[&format!(".claude/skills/zz/SKILL.md: {}", LIMIT + 1)]);
    assert!(!run.output().contains("skills/aa"), "{}", run.output());
}

// A pre-commit hook names in GIT_INDEX_FILE the index the commit is made from, which on
// `git commit <path>` is not the repository's own, and the skills it stages are the ones
// to check.
#[test]
fn checks_the_skills_in_the_index_a_git_hook_names() {
    let hook = TempDir::new().expect("a temporary directory can be made");
    let index = hook.path().join("index");
    Checkout::new(&[(".claude/skills/small/SKILL.md", &skill("a"))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .env("GIT_INDEX_FILE", &index)
        .run()
        .expect(1, &["git tracks no skill"]);
}

// A file that is not UTF-8 has no length in UTF-16 code units, and passing it would
// pass a skill of any length.
#[test]
fn fails_on_a_skill_that_is_not_utf_8() {
    Checkout::holding(".claude/skills/bad/SKILL.md", b"---\nn: x\n---\nab\xffcd\n")
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(1, &[".claude/skills/bad/SKILL.md: it is not UTF-8"]);
}

// What the working tree holds at a tracked skill's path is read only where it is a file,
// as it is where the skill was deleted and a directory made in its place.
#[test]
fn passes_over_a_tracked_skill_whose_path_is_now_a_directory() {
    let checkout = Checkout::new(&[(".claude/skills/aa/SKILL.md", &skill("a"))]);
    checkout.remove(".claude/skills/aa/SKILL.md");
    checkout.make_dir(".claude/skills/aa/SKILL.md");
    let run = checkout.dotfiles_tasks().arg("skills-size-check").run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A skill it cannot read could be of any length.
#[test]
fn fails_on_a_skill_it_cannot_read() {
    let checkout = Checkout::new(&[(".claude/skills/locked/SKILL.md", &skill("a"))]);
    checkout.lock(".claude/skills/locked/SKILL.md");
    checkout
        .dotfiles_tasks()
        .arg("skills-size-check")
        .run()
        .expect(1, &[".claude/skills/locked/SKILL.md: could not be read"]);
}

// A skill in a directory that cannot be searched is not deleted from disk, and passing it
// silently would pass a skill of any length.
#[test]
fn fails_on_a_skill_in_a_directory_it_cannot_search() {
    let checkout = Checkout::new(&[(".claude/skills/locked/SKILL.md", &skill(&over()))]);
    checkout.lock(".claude/skills/locked");
    let run = checkout.dotfiles_tasks().arg("skills-size-check").run();
    checkout.unlock(".claude/skills/locked");
    run.expect(1, &[".claude/skills/locked/SKILL.md"]);
}

// A caller that stops reading the output closes the pipe, and a panic there would hide
// the check's own result behind a backtrace and exit 101.
#[test]
fn does_not_panic_when_stdout_is_closed_while_it_reports_with_verbose() {
    let files: Vec<(String, String)> = (0..8)
        .map(|i| (format!(".claude/skills/s{i}/SKILL.md"), skill("a")))
        .collect();
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, contents)| (path.as_str(), contents.as_str()))
        .collect();
    let run = Checkout::new(&files)
        .dotfiles_tasks()
        .arg("skills-size-check")
        .arg("--verbose")
        .closing(Stream::Stdout)
        .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A caller that reads only stdout may close stderr, and the run still has to end with
// the status that says a skill failed.
#[test]
fn fails_on_a_skill_past_the_limit_when_stderr_is_closed() {
    let run = Checkout::new(&[(".claude/skills/big/SKILL.md", &skill(&over()))])
        .dotfiles_tasks()
        .arg("skills-size-check")
        .closing(Stream::Stderr)
        .run();
    assert_eq!((run.code(), run.output()), (1, ""));
}

// A skill directory replaced by a file leaves no skill at the tracked path, as a
// deleted one does, and the skills after it are still read.
#[test]
fn passes_over_a_tracked_skill_whose_directory_is_now_a_file() {
    let checkout = Checkout::new(&[
        (".claude/skills/aa/SKILL.md", &skill("a")),
        (".claude/skills/zz/SKILL.md", &skill(&over())),
    ]);
    checkout.remove(".claude/skills/aa");
    checkout.write(".claude/skills/aa", "a file");
    let run = checkout.dotfiles_tasks().arg("skills-size-check").run();
    run.expect(1, &[&format!(".claude/skills/zz/SKILL.md: {}", LIMIT + 1)]);
    assert!(!run.output().contains("skills/aa"), "{}", run.output());
}

// An empty pathspec would match every tracked file, so it is read as none given.
#[test]
fn reads_an_empty_pathspec_as_the_default_one() {
    let run = Checkout::new(&[
        (".claude/skills/small/SKILL.md", &skill("a")),
        ("README.md", &over()),
    ])
    .dotfiles_tasks()
    .arg("skills-size-check")
    .arg("")
    .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}
