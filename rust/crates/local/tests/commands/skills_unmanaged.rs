//! `skills-unmanaged`: the entries under `.claude/skills/` that neither git tracks nor
//! apm.lock.yaml deploys, run in a repository of the test's own with the real git and
//! yq, which act on nothing outside it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A git repository of the test's own, which no git config of the machine running the
/// tests reaches.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// A fresh repository named after `test`, holding `lock` as apm.lock.yaml.
    fn new(test: &str, lock: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("skills-unmanaged")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("repo/.claude/skills")).expect("the sandbox is created");
        fs::write(root.join("repo/apm.lock.yaml"), lock).expect("the lockfile is written");
        let sandbox = Sandbox { root };
        sandbox.git(&["init", "--quiet"]);
        sandbox
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    /// Writes `path`, relative to the repository, creating its directories.
    fn file(&self, path: &str) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("the directories are created");
        fs::write(path, "").expect("the file is written");
    }

    /// Writes `path` and has git track it.
    fn tracked(&self, path: &str) {
        self.file(path);
        self.git(&["add", path]);
    }

    fn git(&self, args: &[&str]) {
        let status = self.command("git").args(args).status().expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// `program` run in the repository, with what a git hook sets in the environment
    /// taken away and the machine's git config left unread. HOME stays, since yq may be
    /// a mise shim, which reads the trust mise keeps under it.
    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        command
            .current_dir(self.repo())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.root);
        command
    }

    /// Runs `skills-unmanaged` with `args` from the repository's root.
    fn run(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("skills-unmanaged")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

/// The exit code, stdout and stderr of a run.
fn outcome(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

const NO_DEPENDENCIES: &str = "dependencies: []\n";

#[test]
fn lists_an_entry_neither_tracked_nor_deployed() {
    let sandbox = Sandbox::new("lists-unmanaged", NO_DEPENDENCIES);
    sandbox.file(".claude/skills/local/SKILL.md");

    let run = sandbox.run(&[]);

    assert_eq!(
        outcome(&run),
        (Some(0), ".claude/skills/local\n".to_owned(), String::new())
    );
}

// A file git tracks anywhere under an entry makes the whole entry managed, and an
// untracked file beside it does not make it unmanaged.
#[test]
fn leaves_out_an_entry_git_tracks_a_file_under() {
    let sandbox = Sandbox::new("leaves-out-tracked", NO_DEPENDENCIES);
    sandbox.tracked(".claude/skills/tracked/scripts/run.sh");
    sandbox.file(".claude/skills/tracked/notes.md");
    sandbox.file(".claude/skills/local/SKILL.md");

    let run = sandbox.run(&[]);

    assert_eq!(
        outcome(&run),
        (Some(0), ".claude/skills/local\n".to_owned(), String::new())
    );
}

// A file the lockfile deploys anywhere under an entry makes the whole entry managed,
// whichever dependency lists it, and a dependency listing none deploys nothing.
#[test]
fn leaves_out_an_entry_the_lockfile_deploys_a_file_under() {
    let sandbox = Sandbox::new(
        "leaves-out-deployed",
        "\
dependencies:
- name: listing-none
- name: skill
  deployed_files:
  - .claude/skills/deployed/SKILL.md
- name: gist
  deployed_files:
  - .claude/skills/gist/scripts/run.sh
",
    );
    sandbox.file(".claude/skills/deployed/SKILL.md");
    sandbox.file(".claude/skills/gist/scripts/run.sh");
    sandbox.file(".claude/skills/local/SKILL.md");

    let run = sandbox.run(&[]);

    assert_eq!(
        outcome(&run),
        (Some(0), ".claude/skills/local\n".to_owned(), String::new())
    );
}

// Hidden entries count too, and the list is in byte order, as `sort` orders it.
#[test]
fn lists_every_unmanaged_entry_in_byte_order_hidden_ones_included() {
    let sandbox = Sandbox::new("lists-in-order", NO_DEPENDENCIES);
    for entry in ["beta", "alpha", "Zed", ".hidden"] {
        sandbox.file(&format!(".claude/skills/{entry}/SKILL.md"));
    }
    sandbox.file(".claude/skills/.DS_Store");

    let run = sandbox.run(&[]);

    assert_eq!(
        outcome(&run),
        (
            Some(0),
            "\
.claude/skills/.DS_Store
.claude/skills/.hidden
.claude/skills/Zed
.claude/skills/alpha
.claude/skills/beta
"
            .to_owned(),
            String::new()
        )
    );
}

// Without the lockfile every deployed skill would read as unmanaged, so the run lists
// nothing and fails instead, with yq's own message passed on.
#[test]
fn fails_listing_nothing_where_yq_cannot_read_the_lockfile() {
    let sandbox = Sandbox::new("fails-without-lockfile", NO_DEPENDENCIES);
    fs::remove_file(sandbox.repo().join("apm.lock.yaml")).expect("the lockfile is removed");
    sandbox.file(".claude/skills/local/SKILL.md");

    let (code, stdout, stderr) = outcome(&sandbox.run(&[]));

    assert_eq!((code, stdout.as_str()), (Some(1), ""), "{stderr}");
    assert!(
        stderr.contains("apm.lock.yaml: no such file or directory")
            && stderr.ends_with("yq could not read apm.lock.yaml (exit 1)\n"),
        "{stderr}"
    );
}

#[test]
fn fails_listing_nothing_outside_a_git_repository() {
    let sandbox = Sandbox::new("fails-outside-git", NO_DEPENDENCIES);
    fs::remove_dir_all(sandbox.repo().join(".git")).expect("the repository is removed");
    sandbox.file(".claude/skills/local/SKILL.md");

    let (code, stdout, stderr) = outcome(&sandbox.run(&[]));

    assert_eq!((code, stdout.as_str()), (Some(128), ""), "{stderr}");
    assert!(
        stderr.ends_with("git could not list .claude/skills (exit 128)\n"),
        "{stderr}"
    );
}

#[test]
fn fails_where_there_is_no_skills_directory() {
    let sandbox = Sandbox::new("fails-without-skills", NO_DEPENDENCIES);
    fs::remove_dir(sandbox.repo().join(".claude/skills")).expect("the directory is removed");

    let (code, stdout, stderr) = outcome(&sandbox.run(&[]));

    assert_eq!((code, stdout.as_str()), (Some(1), ""), "{stderr}");
    assert!(
        stderr.starts_with("cannot read .claude/skills: "),
        "{stderr}"
    );
}

#[test]
fn prints_its_usage_on_help() {
    let sandbox = Sandbox::new("prints-usage", NO_DEPENDENCIES);

    let (code, stdout, stderr) = outcome(&sandbox.run(&["--help"]));

    assert_eq!((code, stderr.as_str()), (Some(0), ""));
    assert!(
        stdout.starts_with("Usage: mise run skills:unmanaged\n"),
        "{stdout}"
    );
}

// It takes no arguments, so one is a mistake rather than something to ignore.
#[test]
fn fails_with_its_usage_on_an_argument() {
    let sandbox = Sandbox::new("fails-on-argument", NO_DEPENDENCIES);
    sandbox.file(".claude/skills/local/SKILL.md");

    let (code, stdout, stderr) = outcome(&sandbox.run(&["local"]));

    assert_eq!((code, stdout.as_str()), (Some(2), ""));
    assert!(
        stderr.starts_with("Usage: mise run skills:unmanaged\n"),
        "{stderr}"
    );
}

// git quotes a name with bytes outside ASCII in its line output, which would read as
// another name; the name itself is what counts.
#[test]
fn leaves_out_a_tracked_entry_whose_name_is_not_ascii() {
    let sandbox = Sandbox::new("leaves-out-non-ascii", NO_DEPENDENCIES);
    sandbox.tracked(".claude/skills/技能/SKILL.md");

    let run = sandbox.run(&[]);

    assert_eq!(outcome(&run), (Some(0), String::new(), String::new()));
}
