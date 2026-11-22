//! `skills-push`: pushes a gist-sourced skill's local SKILL.md back to its gist and
//! moves the pin to it, run with the real git against a checkout of the test's own, and
//! with yq and jq failing wherever they are run. git's config sends every gist URL to a
//! bare repository in the sandbox, and a fake apm and gh stand in for theirs, so nothing
//! reaches gist.github.com.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, Sandbox, git, isolated};

/// The gist the test's skill comes from, as apm.lock.yaml names it.
const GIST: &str = "tester/0123abcd";

/// The skill the tests push.
const SKILL: &str = "writing";

/// What stands for the pinned commit in the lockfile the fake apm writes.
const PIN: &str = "@PIN@";

/// A dotfiles checkout of the test's own, a bare repository standing in for the gist,
/// a directory holding the fakes and the records they leave, none of which the
/// machine's git config reaches.
struct Fixture {
    sandbox: Sandbox,
}

impl Fixture {
    /// A fresh sandbox named after `test`: a gist holding one SKILL.md commit, and a
    /// checkout that marks itself as dotfiles, pins that commit in apm.lock.yaml and
    /// holds an edited SKILL.md.
    fn new(test: &str) -> Self {
        let fixture = Fixture {
            sandbox: Sandbox::new("skills-push", test, &["repo", "seed"]),
        };
        fixture.sandbox.forbid(&["yq", "jq"]);
        fixture.write_config();
        fixture.fake_apm();
        fixture.fake_gh();
        let pinned = fixture.seed_gist();
        fixture.write(
            "rust/crates/local/Cargo.toml",
            "[package]\nname = \"dotfiles-local\"\n",
        );
        fixture.write("apm.lock.yaml", &lockfile(&pinned));
        fixture.write(&format!(".claude/skills/{SKILL}/SKILL.md"), "edited\n");
        fixture.git(&fixture.repo(), &["init", "--quiet"]);
        fixture
    }

    /// Writes the git config the tests read in place of the machine's: an identity,
    /// and every gist URL sent to the sandbox's `gists/` directory.
    fn write_config(&self) {
        let config = format!(
            "[user]\n\tname = Test\n\temail = test@example.com\n[url \"file://{}/\"]\n\tinsteadOf = https://gist.github.com/\n",
            self.root().join("gists").display()
        );
        fs::write(self.root().join("gitconfig"), config).expect("the git config is written");
    }

    /// Creates the bare repository standing in for the gist, holding one commit with
    /// SKILL.md, and returns that commit.
    fn seed_gist(&self) -> String {
        let gist = self.gist();
        fs::create_dir_all(&gist).expect("the gist directory is created");
        self.git(&gist, &["init", "--quiet", "--bare"]);
        let seed = self.root().join("seed");
        self.git(&seed, &["init", "--quiet"]);
        fs::write(seed.join("SKILL.md"), "pinned\n").expect("the seed is written");
        self.git(&seed, &["add", "SKILL.md"]);
        self.git(&seed, &["commit", "--quiet", "--message", "first"]);
        self.git(
            &seed,
            &[
                "push",
                "--quiet",
                gist.to_str().expect("UTF-8"),
                "HEAD:main",
            ],
        );
        self.git(&gist, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        self.read_git(&gist, &["rev-parse", "HEAD"])
    }

    /// Writes a fake apm into the sandbox's PATH, which records its working directory
    /// and arguments, then writes the test's lockfile pinning the gist's head, or the
    /// commit in `apm-pin` where there is one, and exits with the code in `apm-code`, if
    /// any.
    fn fake_apm(&self) {
        fs::write(self.root().join("apm-lockfile"), lockfile(PIN))
            .expect("the lockfile apm writes is written");
        self.sandbox.fake(
            "apm",
            &format!(
                r#"{{ pwd; printf '%s\n' "$@"; }} > "$SANDBOX/apm-call"
code=$(cat "$SANDBOX/apm-code" 2>/dev/null || echo 0)
[ "$code" = 0 ] || exit "$code"
pin=$(cat "$SANDBOX/apm-pin" 2>/dev/null || git -C "$SANDBOX/gists/tester/0123abcd.git" rev-parse HEAD)
sed "s/{PIN}/$pin/" "$SANDBOX/apm-lockfile" > apm.lock.yaml"#
            ),
        );
    }

    /// Writes a fake gh into the sandbox's PATH, which records its arguments and gives
    /// no credential.
    fn fake_gh(&self) {
        self.sandbox.fake(
            "gh",
            "printf '%s\\n' \"$@\" >> \"$SANDBOX/gh-calls\"\nexit 1",
        );
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// The working directory and arguments apm was called with, one a line, or `None`
    /// where it was not called.
    fn apm_call(&self) -> Option<String> {
        fs::read_to_string(self.root().join("apm-call")).ok()
    }

    fn gist(&self) -> PathBuf {
        self.root().join("gists").join(format!("{GIST}.git"))
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    /// Writes `contents` to the checkout's `path`, creating its directories.
    fn write(&self, path: &str, contents: &str) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("the directories are created");
        fs::write(path, contents).expect("the file is written");
    }

    /// What the checkout's `path` holds.
    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.repo().join(path)).expect("the file is read")
    }

    /// Runs git with `args` in `dir`.
    fn git(&self, dir: &Path, args: &[&str]) {
        let status = self
            .command(git(), dir)
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// What git with `args` in `dir` prints, its trailing newline cut.
    fn read_git(&self, dir: &Path, args: &[&str]) -> String {
        let output = self
            .command(git(), dir)
            .args(args)
            .output()
            .expect("git starts");
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    }

    /// `program` run in `dir`, with what a git hook sets in the environment taken away,
    /// the sandbox's git config read in place of the machine's, and the fakes first on
    /// PATH.
    fn command(&self, program: impl AsRef<OsStr>, dir: &Path) -> Command {
        let mut command = isolated(program, dir, self.root());
        command
            .env("PATH", self.sandbox.path_before_inherited())
            .env("SANDBOX", self.root())
            .env("GIT_CONFIG_GLOBAL", self.root().join("gitconfig"))
            .env("GIT_TERMINAL_PROMPT", "0");
        command
    }

    /// Runs `skills-push` with `args` in `dir`.
    fn run(&self, dir: &Path, args: &[&str]) -> Output {
        self.command(DOTFILES_LOCAL, dir)
            .arg("skills-push")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

/// An apm.lock.yaml pinning the skill's gist at `pinned`, after a GitHub dependency of
/// the same name and a gist dependency of another, neither of which is the one pushed.
fn lockfile(pinned: &str) -> String {
    format!(
        "lockfile_version: '1'
dependencies:
- repo_url: tester/skills
  name: {SKILL}
  host: github.com
  resolved_commit: 1111111111111111111111111111111111111111
- repo_url: tester/fedcba98
  alias: other
  name: other
  host: gist.github.com
  resolved_commit: 3333333333333333333333333333333333333333
- repo_url: {GIST}
  alias: {SKILL}
  name: {SKILL}
  host: gist.github.com
  resolved_commit: {pinned}
  deployed_files:
  - .claude/skills/{SKILL}
  - .claude/skills/{SKILL}/SKILL.md
"
    )
}

/// The exit code, stdout and stderr of a run.
fn outcome(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn pushes_the_local_skill_on_top_of_the_pin_and_pins_the_push() {
    let sandbox = Fixture::new("pushes");
    let gist = sandbox.gist();
    let pinned = sandbox.read_git(&gist, &["rev-parse", "HEAD"]);

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    let pushed = sandbox.read_git(&gist, &["rev-parse", "HEAD"]);
    assert_eq!(
        outcome(&run),
        (
            Some(0),
            format!(
                "[ok] {SKILL} -> gist {GIST} at {pushed}\n[note] commit apm.lock.yaml and land it on main, or an install from main restores the old copy\n"
            ),
            String::new()
        )
    );
    assert_eq!(
        sandbox.read_git(&gist, &["rev-parse", "HEAD^"]),
        pinned,
        "the push sits on the pin"
    );
    assert_eq!(
        sandbox.read_git(&gist, &["show", "HEAD:SKILL.md"]),
        "edited"
    );
    assert_eq!(
        sandbox.read_git(&gist, &["log", "-1", "--format=%s"]),
        format!("Update {SKILL}")
    );
    let repo = fs::canonicalize(sandbox.repo()).expect("the checkout resolves");
    assert_eq!(
        sandbox.apm_call(),
        Some(format!("{}\nupdate\n--yes\n{SKILL}\n", repo.display()))
    );
    assert!(sandbox.read("apm.lock.yaml").contains(&pushed));
}

// A pin other than the pushed commit would restore another revision on the next
// install.
#[test]
fn fails_where_apm_pins_another_commit() {
    let sandbox = Fixture::new("pins-another");
    let other = "2222222222222222222222222222222222222222";
    fs::write(sandbox.root().join("apm-pin"), other).expect("the pin is written");

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    let pushed = sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]);
    assert_eq!(
        outcome(&run),
        (
            Some(1),
            format!("[ok] {SKILL} -> gist {GIST} at {pushed}\n"),
            format!(
                "[fail] {SKILL}: pushed {pushed} but apm.lock.yaml pins {other}; rerun 'apm update --yes {SKILL}'\n"
            )
        )
    );
}

// The commit goes on top of the pinned revision, so a gist that has moved past it
// rejects the push rather than losing that revision, and the local SKILL.md, the
// edit's only copy, is left for apm not to overwrite.
#[test]
fn fails_without_updating_where_the_gist_moved_past_the_pin() {
    let sandbox = Fixture::new("moved-past");
    let seed = sandbox.root().join("seed");
    fs::write(seed.join("SKILL.md"), "moved\n").expect("the seed is written");
    sandbox.git(
        &seed,
        &["commit", "--quiet", "--all", "--message", "second"],
    );
    let gist = sandbox.gist();
    sandbox.git(
        &seed,
        &[
            "push",
            "--quiet",
            gist.to_str().expect("UTF-8"),
            "HEAD:main",
        ],
    );
    let moved = sandbox.read_git(&gist, &["rev-parse", "HEAD"]);

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    let (code, stdout, stderr) = outcome(&run);
    assert_eq!((code, stdout.as_str()), (Some(1), ""));
    assert!(
        stderr.ends_with(&format!(
            "[fail] {SKILL}: push refused (see git's message above); if the gist moved past the pin, copy .claude/skills/{SKILL}/SKILL.md aside, run 'apm update --yes {SKILL}', redo the edit on top and rerun\n"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("rejected"), "git says why: {stderr}");
    assert_eq!(sandbox.read_git(&gist, &["rev-parse", "HEAD"]), moved);
    assert_eq!(sandbox.apm_call(), None);
}

// The name becomes a path component.
#[test]
fn refuses_a_name_that_is_not_one_path_component() {
    let sandbox = Fixture::new("invalid-name");
    for (name, shown) in [("", "<empty>"), (".", "."), ("..", ".."), ("a/b", "a/b")] {
        let run = sandbox.run(&sandbox.repo(), &[name]);

        assert_eq!(
            outcome(&run),
            (
                Some(1),
                String::new(),
                format!("[fail] {shown}: invalid skill name\n")
            ),
            "{name:?}"
        );
    }
    assert_eq!(sandbox.apm_call(), None);
}

// Only a dependency from gist.github.com counts, not one of the same name elsewhere.
#[test]
fn fails_where_no_gist_dependency_has_the_name() {
    let sandbox = Fixture::new("no-gist");
    let lock = sandbox
        .read("apm.lock.yaml")
        .replace("host: gist.github.com", "host: github.com");
    sandbox.write("apm.lock.yaml", &lock);

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    assert_eq!(
        outcome(&run),
        (
            Some(1),
            String::new(),
            format!("[fail] {SKILL}: no gist dependency by that alias in apm.lock.yaml\n")
        )
    );
    assert_eq!(sandbox.apm_call(), None);
}

// A gist's repo_url is `<owner>/<gist_id>`, so one of another form, which would reach
// `git clone` as part of a URL, is not taken for the gist.
#[test]
fn fails_where_the_gist_dependency_names_no_gist() {
    for repo in ["tester/not-hex", "../x"] {
        let sandbox = Fixture::new(&format!("not-a-gist-{}", repo.replace('/', "-")));
        let lock = sandbox
            .read("apm.lock.yaml")
            .replace(&format!("repo_url: {GIST}"), &format!("repo_url: {repo}"));
        sandbox.write("apm.lock.yaml", &lock);

        let run = sandbox.run(&sandbox.repo(), &[SKILL]);

        assert_eq!(
            outcome(&run),
            (
                Some(1),
                String::new(),
                format!("[fail] {SKILL}: no gist dependency by that alias in apm.lock.yaml\n")
            ),
            "{repo}"
        );
        assert_eq!(sandbox.apm_call(), None, "{repo}");
    }
}

#[test]
fn fails_where_the_skill_has_no_local_copy() {
    let sandbox = Fixture::new("no-local");
    fs::remove_file(
        sandbox
            .repo()
            .join(format!(".claude/skills/{SKILL}/SKILL.md")),
    )
    .expect("the copy is removed");

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    assert_eq!(
        outcome(&run),
        (
            Some(1),
            String::new(),
            format!(
                "[fail] {SKILL}: no local SKILL.md at .claude/skills/{SKILL}/SKILL.md (run 'apm install' first)\n"
            )
        )
    );
    assert_eq!(sandbox.apm_call(), None);
}

// Run in another repository, the command would push that repository's gist; the
// checkout holding dotfiles-local's source is the one it acts for.
#[test]
fn fails_without_pushing_outside_a_dotfiles_checkout() {
    let sandbox = Fixture::new("not-dotfiles");
    let pinned = sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]);
    for manifest in [None, Some("[package]\nname = \"other\"\n")] {
        let path = sandbox.repo().join("rust/crates/local/Cargo.toml");
        match manifest {
            None => fs::remove_file(&path).expect("the manifest is removed"),
            Some(contents) => fs::write(&path, contents).expect("the manifest is written"),
        }

        let run = sandbox.run(&sandbox.repo(), &[SKILL]);

        let repo = fs::canonicalize(sandbox.repo()).expect("the checkout resolves");
        assert_eq!(
            outcome(&run),
            (
                Some(1),
                String::new(),
                format!(
                    "[fail] {} is not a dotfiles checkout: no rust/crates/local/Cargo.toml naming dotfiles-local\n",
                    repo.display()
                )
            ),
            "{manifest:?}"
        );
    }
    assert_eq!(
        sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]),
        pinned
    );
    assert_eq!(sandbox.apm_call(), None);
}

// The checkout acted on is the one the command runs in, from any directory under it:
// run from a worktree, the main checkout's copy and pin stay as they are.
#[test]
fn acts_on_the_worktree_it_runs_in() {
    let sandbox = Fixture::new("worktree");
    let repo = sandbox.repo();
    sandbox.git(&repo, &["add", "--all"]);
    sandbox.git(&repo, &["commit", "--quiet", "--message", "checkout"]);
    let worktree = sandbox.root().join("worktree");
    sandbox.git(
        &repo,
        &[
            "worktree",
            "add",
            "--quiet",
            worktree.to_str().expect("UTF-8"),
            "-b",
            "side",
        ],
    );
    let skill = format!(".claude/skills/{SKILL}/SKILL.md");
    fs::write(worktree.join(&skill), "from the worktree\n").expect("the copy is written");
    let main_lock = sandbox.read("apm.lock.yaml");

    let run = sandbox.run(&worktree.join(".claude"), &[SKILL]);

    assert_eq!(run.status.code(), Some(0), "{}", outcome(&run).2);
    let gist = sandbox.gist();
    assert_eq!(
        sandbox.read_git(&gist, &["show", "HEAD:SKILL.md"]),
        "from the worktree"
    );
    let worktree = fs::canonicalize(&worktree).expect("the worktree resolves");
    assert_eq!(
        sandbox.apm_call(),
        Some(format!("{}\nupdate\n--yes\n{SKILL}\n", worktree.display()))
    );
    let pushed = sandbox.read_git(&gist, &["rev-parse", "HEAD"]);
    let worktree_lock =
        fs::read_to_string(worktree.join("apm.lock.yaml")).expect("the lockfile is read");
    assert!(worktree_lock.contains(&pushed));
    assert_eq!(sandbox.read("apm.lock.yaml"), main_lock);
}

#[test]
fn ends_with_apms_code_where_apm_fails() {
    let sandbox = Fixture::new("apm-fails");
    fs::write(sandbox.root().join("apm-code"), "3").expect("the code is written");

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    let pushed = sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]);
    assert_eq!(
        outcome(&run),
        (
            Some(3),
            format!("[ok] {SKILL} -> gist {GIST} at {pushed}\n"),
            String::new()
        )
    );
}

#[test]
fn prints_its_usage_on_help() {
    let sandbox = Fixture::new("help");
    for flag in ["--help", "-h"] {
        let (code, stdout, stderr) = outcome(&sandbox.run(&sandbox.repo(), &[flag]));

        assert_eq!((code, stderr.as_str()), (Some(0), ""), "{flag}");
        assert!(
            stdout.contains("Usage: dotfiles-local skills-push <NAME>\n"),
            "{flag}: {stdout}"
        );
    }
    assert_eq!(sandbox.apm_call(), None);
}

#[test]
fn fails_as_a_usage_error_on_other_than_one_name() {
    let sandbox = Fixture::new("usage");
    for args in [&[][..], &[SKILL, "other"][..]] {
        let (code, stdout, stderr) = outcome(&sandbox.run(&sandbox.repo(), args));

        assert_eq!((code, stdout.as_str()), (Some(2), ""), "{args:?}");
        assert!(
            stderr.starts_with("error: ")
                && stderr.contains("Usage: dotfiles-local skills-push <NAME>\n"),
            "{args:?}: {stderr}"
        );
    }
    assert_eq!(sandbox.apm_call(), None);
}

// An unchanged copy leaves nothing to commit, and so nothing to push or pin.
#[test]
fn fails_where_the_local_copy_matches_the_pin() {
    let sandbox = Fixture::new("unchanged");
    sandbox.write(&format!(".claude/skills/{SKILL}/SKILL.md"), "pinned\n");
    let pinned = sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]);

    let run = sandbox.run(&sandbox.repo(), &[SKILL]);

    let (code, stdout, stderr) = outcome(&run);
    assert_eq!((code, stderr.as_str()), (Some(1), ""));
    assert!(
        stdout.ends_with("nothing to commit, working tree clean\n"),
        "git says why: {stdout}"
    );
    assert!(!stdout.contains("[ok]"), "{stdout}");
    assert_eq!(
        sandbox.read_git(&sandbox.gist(), &["rev-parse", "HEAD"]),
        pinned
    );
    assert_eq!(sandbox.apm_call(), None);
}
