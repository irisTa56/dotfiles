//! `dotfiles-tasks size-check`: whether the body of each skill git tracks fits in what a
//! compaction re-attaches.

use tempfile::TempDir;

use crate::support::checkout::Checkout;

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
        .size_check(&[])
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
        .size_check(&[])
        .expect(1, &[&format!(".claude/skills/e/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn passes_a_body_at_the_limit_printing_nothing() {
    let run = Checkout::new(&[(".claude/skills/full/SKILL.md", &skill(&"a".repeat(LIMIT)))])
        .size_check(&[]);
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A session sizing a skill against the limit needs the length of one that passes.
#[test]
fn with_verbose_reports_a_passing_skills_length_against_the_limit() {
    let run = Checkout::new(&[(".claude/skills/full/SKILL.md", &skill(&"a".repeat(LIMIT)))])
        .size_check(&["--verbose"]);
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
    .size_check(&["--verbose", ":(glob)skills/*/SKILL.md"])
    .expect(
        1,
        &[
            &format!("skills/big/SKILL.md: {}", LIMIT + 1),
            &format!("skills/small/SKILL.md: 1 of {LIMIT}"),
        ],
    );
}

// An argument after the pathspec is not read, and dropping it silently would leave the
// caller with no lengths and no reason.
#[test]
fn fails_naming_an_argument_it_does_not_read_rather_than_dropping_it() {
    Checkout::new(&[(".claude/skills/small/SKILL.md", &skill("a"))])
        .size_check(&[":(glob).claude/skills/*/SKILL.md", "--verbose"])
        .expect(2, &["does not read --verbose"]);
}

// Run where git lists nothing, the check would otherwise pass having read no skill.
#[test]
fn fails_when_git_tracks_no_skill_to_check() {
    Checkout::new(&[])
        .size_check(&[])
        .expect(1, &["git tracks no skill"]);
}

#[test]
fn fails_with_gits_status_where_git_cannot_list_the_skills() {
    let checkout = Checkout::new(&[]);
    checkout.remove(".git");
    checkout
        .size_check(&[])
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
    .size_check(&[]);
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn checks_only_the_skill_md_one_level_under_claude_skills() {
    let run = Checkout::new(&[
        (".claude/skills/x/SKILL.md", &skill("a")),
        (".claude/skills/x/fixtures/y/SKILL.md", &skill(&over())),
    ])
    .size_check(&[]);
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn checks_the_skills_a_pathspec_argument_names_in_place_of_claude_skills() {
    let run = Checkout::new(&[
        ("skills/big/SKILL.md", &skill(&over())),
        (".claude/skills/small/SKILL.md", &skill(&over())),
    ])
    .size_check(&[":(glob)skills/*/SKILL.md"]);
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
        .size_check(&[]);
    assert_eq!((run.code(), run.output()), (0, ""));
}

#[test]
fn reports_a_skill_past_the_limit_behind_a_crlf_frontmatter() {
    Checkout::new(&[(".claude/skills/crlf/SKILL.md", &crlf(&over()))])
        .size_check(&[])
        .expect(
            1,
            &[&format!(".claude/skills/crlf/SKILL.md: {}", LIMIT + 1)],
        );
}

// With no frontmatter, the whole file is the body: a `---` rule in it is no fence.
#[test]
fn counts_the_whole_of_a_skill_with_no_frontmatter() {
    Checkout::new(&[(".claude/skills/bare/SKILL.md", &over())])
        .size_check(&[])
        .expect(
            1,
            &[&format!(".claude/skills/bare/SKILL.md: {}", LIMIT + 1)],
        );
}

#[test]
fn counts_the_whole_of_a_skill_with_no_frontmatter_and_a_rule_line_in_it() {
    let contents = format!("{}\n---\nshort tail\n", over());
    Checkout::new(&[(".claude/skills/rule/SKILL.md", &contents)])
        .size_check(&[])
        .expect(
            1,
            &[&format!(".claude/skills/rule/SKILL.md: {}", LIMIT + 1 + 16)],
        );
}

#[test]
fn fails_on_a_frontmatter_that_never_closes_rather_than_reading_it_as_all_frontmatter() {
    let contents = format!("---\nname: x\n{}", over());
    Checkout::new(&[(".claude/skills/open/SKILL.md", &contents)])
        .size_check(&[])
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
    .size_check(&[])
    .expect(1, &[&format!(".claude/skills/zz/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn reports_a_skill_past_the_limit_under_a_non_ascii_directory() {
    Checkout::new(&[(".claude/skills/é/SKILL.md", &skill(&over()))])
        .size_check(&[])
        .expect(1, &[&format!(".claude/skills/é/SKILL.md: {}", LIMIT + 1)]);
}

#[test]
fn passes_a_short_skill_under_a_non_ascii_directory() {
    let run = Checkout::new(&[(".claude/skills/é/SKILL.md", &skill("a"))]).size_check(&[]);
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
    let run = checkout.size_check(&[]);
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
        .size_check_with(&[], &[("GIT_INDEX_FILE", Some(index.as_os_str()))])
        .expect(1, &["git tracks no skill"]);
}

// A file that is not UTF-8 has no length in UTF-16 code units, and passing it would
// pass a skill of any length.
#[test]
fn fails_on_a_skill_that_is_not_utf_8() {
    Checkout::holding(".claude/skills/bad/SKILL.md", b"---\nn: x\n---\nab\xffcd\n")
        .size_check(&[])
        .expect(1, &[".claude/skills/bad/SKILL.md: it is not UTF-8"]);
}

// What the working tree holds at a tracked skill's path is read only where it is a file,
// as it is where the skill was deleted and a directory made in its place.
#[test]
fn passes_over_a_tracked_skill_whose_path_is_now_a_directory() {
    let checkout = Checkout::new(&[(".claude/skills/aa/SKILL.md", &skill("a"))]);
    checkout.remove(".claude/skills/aa/SKILL.md");
    checkout.make_dir(".claude/skills/aa/SKILL.md");
    let run = checkout.size_check(&[]);
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A skill it cannot read could be of any length.
#[test]
fn fails_on_a_skill_it_cannot_read() {
    let checkout = Checkout::new(&[(".claude/skills/locked/SKILL.md", &skill("a"))]);
    checkout.lock(".claude/skills/locked/SKILL.md");
    checkout
        .size_check(&[])
        .expect(1, &[".claude/skills/locked/SKILL.md: could not be read"]);
}

// An empty pathspec would match every tracked file, so it is read as none given.
#[test]
fn reads_an_empty_pathspec_as_the_default_one() {
    let run = Checkout::new(&[
        (".claude/skills/small/SKILL.md", &skill("a")),
        ("README.md", &over()),
    ])
    .size_check(&[""]);
    assert_eq!((run.code(), run.output()), (0, ""));
}
