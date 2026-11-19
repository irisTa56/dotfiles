//! Runs through `tasks/skills/size-check.sh`, the wrapper consumers run, which installs
//! the release it pins with mise and runs it.

use crate::support::checkout::Checkout;
use crate::support::mise::Mise;

/// A `SKILL.md` with a frontmatter, followed by `body`.
fn skill(body: &str) -> String {
    format!("---\nname: x\ndescription: y\n---\n{body}")
}

// Installing the release can fail, for a version not yet published or a host no binary
// is built for, and that is no answer about the skills.
#[test]
fn fails_as_unable_to_check_where_the_release_it_pins_cannot_be_installed() {
    let run = Checkout::new(&[(".claude/skills/small/SKILL.md", &skill("a"))])
        .size_check_wrapper(&[], &Mise::installing_with(1));
    assert_eq!((run.code(), run.output()), (2, ""));
}

// A consumer names its skills with a pathspec, which reaches the binary only as written:
// one holding a space, split in two, would be read as a pathspec and an argument past it.
#[test]
fn passes_its_arguments_to_size_check_as_given() {
    let run = Checkout::new(&[("my skills/a/SKILL.md", &skill("a"))]).size_check_wrapper(
        &["--verbose", ":(glob)my skills/*/SKILL.md"],
        &Mise::installing_with(0),
    );
    assert_eq!(
        (run.code(), run.output()),
        (0, "my skills/a/SKILL.md: 1 of 19800 UTF-16 code units\n")
    );
}

// The binary's exit code is the consumer's answer, so the wrapper must end with it.
#[test]
fn exits_as_size_check_does_on_a_skill_past_the_limit() {
    Checkout::new(&[(".claude/skills/big/SKILL.md", &skill(&"a".repeat(19_801)))])
        .size_check_wrapper(&[], &Mise::installing_with(0))
        .expect(1, &[".claude/skills/big/SKILL.md: 19801 UTF-16 code units"]);
}
