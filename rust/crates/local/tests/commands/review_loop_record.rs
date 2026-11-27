//! `review-loop-record-paths` and `review-loop-record-selector-input`: the review-loop
//! record of the branch checked out in a repository of the test's own, under a home
//! directory of the test's own.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, IDENTITY, Sandbox, git, isolated};

/// A repository on a branch whose name holds a `/`, as a worktree's branch does, and a
/// home directory beside it.
struct Fixture {
    sandbox: Sandbox,
}

/// The branch the fixture's repository has checked out.
const BRANCH: &str = "claude/some-change";

impl Fixture {
    fn new(test: &str) -> Self {
        let sandbox = Sandbox::new("review-loop-record", test, &["repo", "home", "tmp"]);
        let fixture = Fixture { sandbox };
        fixture.git(&["init", "--quiet", "--initial-branch", BRANCH]);
        fixture.git(&["commit", "--quiet", "--allow-empty", "--message", "start"]);
        fixture
    }

    fn repo(&self) -> PathBuf {
        self.sandbox.root().join("repo")
    }

    fn home(&self) -> PathBuf {
        self.sandbox.root().join("home")
    }

    fn tmp(&self) -> PathBuf {
        self.sandbox.root().join("tmp")
    }

    /// The directory name the record's two directories take for this repository: the
    /// absolute path of its git directory with each `/` as `-`.
    fn key(&self) -> String {
        fs::canonicalize(self.repo().join(".git"))
            .expect("the git directory resolves")
            .to_string_lossy()
            .replace('/', "-")
    }

    fn record_dir(&self) -> PathBuf {
        self.home().join(".claude/review-loop").join(self.key())
    }

    fn background_dir(&self) -> PathBuf {
        self.home()
            .join(".claude/review-loop-background")
            .join(self.key())
    }

    fn record(&self) -> PathBuf {
        self.record_dir().join(format!("{BRANCH}.md"))
    }

    fn background(&self) -> PathBuf {
        self.background_dir().join(format!("{BRANCH}.md"))
    }

    fn git(&self, args: &[&str]) {
        let status = isolated(git(), &self.repo(), self.sandbox.root())
            .args(args)
            .envs(IDENTITY)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// Runs `command` from the repository's root, with the fixture's home and temporary
    /// directory.
    fn run(&self, command: &str) -> Output {
        let mut run: Command = isolated(DOTFILES_LOCAL, &self.repo(), self.sandbox.root());
        run.arg(command)
            .env("HOME", self.home())
            .env("TMPDIR", self.tmp())
            .output()
            .expect("dotfiles-local starts")
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// The paths are printed for the loop to copy rather than compose, and the directories
// exist once they are, so the loop can write the files at once.
#[test]
fn paths_prints_both_directories_and_files_and_creates_the_directories() {
    let fixture = Fixture::new("paths");

    let run = fixture.run("review-loop-record-paths");

    assert_eq!(
        (run.status.code(), text(&run.stdout), text(&run.stderr)),
        (
            Some(0),
            format!(
                "record directory: {}\nbackground directory: {}\nrecord: {}\nbackground: {}\n",
                fixture.record_dir().display(),
                fixture.background_dir().display(),
                fixture.record().display(),
                fixture.background().display(),
            ),
            String::new(),
        )
    );
    assert!(
        fixture
            .record()
            .parent()
            .is_some_and(std::path::Path::is_dir)
    );
    assert!(
        fixture
            .background()
            .parent()
            .is_some_and(std::path::Path::is_dir)
    );
}

impl Fixture {
    /// Writes `text` to `path`, creating its directories.
    fn write(path: &PathBuf, text: &str) {
        fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("the directories are created");
        fs::write(path, text).expect("the file is written");
    }

    fn finding_bar(&self) -> PathBuf {
        self.home().join(".claude/skills/finding-bar/SKILL.md")
    }
}

// The selector gets the three files whole, each under the path it came from, in a file
// outside both directories, since finding a record reads every file under them. What a
// shell would expand reaches it as written.
#[test]
fn selector_input_writes_the_background_record_and_finding_bar_whole_into_a_new_file() {
    let fixture = Fixture::new("selector-input");
    Fixture::write(&fixture.background(), "background $0.00 \\v\n");
    Fixture::write(&fixture.record(), "## Verdicts\n\n$(cat x)\n");
    Fixture::write(&fixture.finding_bar(), "---\nname: finding-bar\n---\nbar\n");

    let run = fixture.run("review-loop-record-selector-input");

    let stdout = text(&run.stdout);
    assert_eq!(
        (run.status.code(), text(&run.stderr)),
        (Some(0), String::new()),
        "{stdout}"
    );
    let written = PathBuf::from(stdout.strip_suffix('\n').expect("one line is printed"));
    assert!(
        written.starts_with(fs::canonicalize(fixture.tmp()).expect("tmp resolves"))
            || written.starts_with(fixture.tmp()),
        "{}",
        written.display()
    );
    assert_eq!(
        fs::read_to_string(&written).expect("the input is written"),
        format!(
            "===== {} =====\n\nbackground $0.00 \\v\n\n\
             ===== {} =====\n\n## Verdicts\n\n$(cat x)\n\n\
             ===== {} =====\n\n---\nname: finding-bar\n---\nbar\n\n",
            fixture.background().display(),
            fixture.record().display(),
            fixture.finding_bar().display(),
        )
    );
}

// An empty or missing part would reach the selector as nothing to weigh, which it would
// answer as though the record held nothing open; so the command stops, naming the part,
// and leaves no file for a spawn to be handed.
#[test]
fn selector_input_fails_naming_an_empty_or_missing_part_and_writes_nothing() {
    for (test, empty) in [("empty-background", true), ("missing-background", false)] {
        let fixture = Fixture::new(test);
        if empty {
            Fixture::write(&fixture.background(), "");
        }
        Fixture::write(&fixture.record(), "## Verdicts\n");
        Fixture::write(&fixture.finding_bar(), "bar\n");

        let run = fixture.run("review-loop-record-selector-input");

        let stderr = text(&run.stderr);
        assert_eq!(
            (run.status.code(), text(&run.stdout)),
            (Some(1), String::new()),
            "{test}: {stderr}"
        );
        assert!(
            stderr.contains(&fixture.background().display().to_string()),
            "{test}: {stderr}"
        );
        assert_eq!(
            fs::read_dir(fixture.tmp()).expect("tmp is read").count(),
            0,
            "{test}"
        );
    }
}
