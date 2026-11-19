//! `skills-update`: APM skills updated unless their last change upstream merged under a
//! day ago, run with the real yq reading the lockfile and a fake gh that applies the
//! `--jq` filter it is given with the real jq, as gh does, so the filters are checked
//! here too. apm is a fake that records how it was called.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jiff::{SignedDuration, Timestamp};

/// A merge time long past the day a change is held for.
const OLD: &str = "2026-01-01T00:00:00Z";

const UNLISTED: &str =
    "GitHub lists no merge into the default branch for its last commit; check by hand";

/// The lockfile most tests read: two skills of one repository, and a gist.
const LOCK: &str = "\
dependencies:
- name: old
  host: github.com
  repo_url: o/r
  virtual_path: skills/old
- name: recent
  host: github.com
  repo_url: o/r
  virtual_path: skills/recent
- name: own
  host: gist.github.com
  repo_url: me/abc
";

/// What GitHub answers about a skill's last commit.
enum Answer<'a> {
    /// A pull request merged at this time into the default branch.
    Merged(&'a str),
    /// A pull request merged at this time into a branch other than the default one.
    OtherBranch(&'a str),
    /// No pull request at all.
    NoPull,
    /// No commit on the skill's path, as when upstream moved or deleted it.
    NoCommit,
    /// A request about its pull requests that fails.
    Fail,
}

/// A directory of the test's own, holding the lockfile and the fake gh and apm.
struct Sandbox {
    root: PathBuf,
}

/// What a run left: its exit status and its stdout followed by its stderr.
struct Run {
    code: Option<i32>,
    out: String,
}

impl Run {
    fn lines(&self) -> Vec<&str> {
        self.out.lines().collect()
    }
}

impl Sandbox {
    /// A fresh sandbox named after `test`, holding `lock` as apm.lock.yaml, whose apm
    /// fails with `exit` when called with the subcommand `failing`, if any.
    fn new(test: &str, lock: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("skills-update")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).expect("the sandbox is created");
        fs::create_dir_all(root.join("gh")).expect("the GitHub answers' directory is created");
        fs::write(root.join("apm.lock.yaml"), lock).expect("the lockfile is written");
        let sandbox = Sandbox { root };
        sandbox.apm_failing("", 0);
        // It answers a path with the file named after it, through jq as gh applies
        // --jq, and fails where there is none. It records each path it is asked.
        sandbox.fake(
            "gh",
            "dir=$(dirname \"$0\")/..\n\
             echo \"$2\" >> \"$dir/gh.log\"\n\
             answer=\"$dir/gh/$(printf '%s' \"$2\" | tr '/?&=' '____')\"\n\
             [ -f \"$answer\" ] || exit 1\n\
             exec jq -r \"$4\" \"$answer\"\n",
        );
        sandbox
    }

    fn fake(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
    }

    /// Makes apm fail with `code` when its subcommand is `subcommand`.
    fn apm_failing(&self, subcommand: &str, code: u8) {
        self.fake(
            "apm",
            &format!(
                "dir=$(dirname \"$0\")/..\n\
                 echo \"$*\" >> \"$dir/apm.log\"\n\
                 pwd -P >> \"$dir/apm.cwd\"\n\
                 echo \"apm says $1\"\n\
                 echo \"apm warns $1\" >&2\n\
                 [ \"$1\" = '{subcommand}' ] && exit {code}\n\
                 exit 0\n"
            ),
        );
    }

    /// Makes GitHub give `answer` about the last commit on `path` in the repository `repo`,
    /// or on the whole repository where `path` is `None`.
    fn github(&self, repo: &str, path: Option<&str>, answer: &Answer<'_>) {
        let query = path.map_or_else(String::new, |path| format!("path={path}&"));
        let sha = format!("sha-{}", path.unwrap_or(repo).replace('/', "-"));
        let commits = if let Answer::NoCommit = answer {
            "[]".to_owned()
        } else {
            format!(r#"[{{"sha": "{sha}"}}]"#)
        };
        self.answer(&format!("repos/{repo}/commits?{query}per_page=1"), &commits);
        let pulls = match answer {
            Answer::Merged(date) => pulls(date, "main"),
            Answer::OtherBranch(date) => pulls(date, "dev"),
            Answer::NoPull => "[]".to_owned(),
            Answer::NoCommit | Answer::Fail => return,
        };
        self.answer(&format!("repos/{repo}/commits/{sha}/pulls"), &pulls);
    }

    /// Makes GitHub give `answer` about a skill under `skills/` of `o/r`.
    fn skill(&self, name: &str, answer: &Answer<'_>) {
        self.github("o/r", Some(&format!("skills/{name}")), answer);
    }

    fn answer(&self, path: &str, json: &str) {
        let file: String = path
            .chars()
            .map(|c| if "/?&=".contains(c) { '_' } else { c })
            .collect();
        fs::write(self.root.join("gh").join(file), json).expect("the answer is written");
    }

    /// Runs `skills-update` with `args` from the sandbox, finding the fakes first on PATH.
    fn run(&self, args: &[&str]) -> Run {
        let output: Output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("skills-update")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", self.search_path())
            .output()
            .expect("dotfiles-local starts");
        Run {
            code: output.status.code(),
            out: format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }

    /// The tests' own PATH, after the sandbox's `bin`.
    fn search_path(&self) -> std::ffi::OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        std::env::join_paths(
            std::iter::once(self.root.join("bin")).chain(std::env::split_paths(&inherited)),
        )
        .expect("PATH is joined")
    }

    /// Each call apm was given, its arguments joined by spaces.
    fn apm_calls(&self) -> Vec<String> {
        self.log("apm.log")
    }

    /// Each path gh was asked about.
    fn gh_calls(&self) -> Vec<String> {
        self.log("gh.log")
    }

    fn log(&self, name: &str) -> Vec<String> {
        // A fake writes its log on its first call.
        fs::read_to_string(self.root.join(name)).map_or_else(
            |_| Vec::new(),
            |log| log.lines().map(str::to_owned).collect(),
        )
    }
}

fn pulls(date: &str, base: &str) -> String {
    format!(
        r#"[{{"merged_at": "{date}", "base": {{"ref": "{base}", "repo": {{"default_branch": "main"}}}}}}]"#
    )
}

fn hours_ago(hours: i64) -> String {
    (Timestamp::now() - SignedDuration::from_hours(hours)).to_string()
}

#[test]
fn updates_a_skill_whose_last_change_merged_over_a_day_ago_and_holds_a_newer_one() {
    let sandbox = Sandbox::new("updates-and-holds", LOCK);
    let recent = hours_ago(23);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(&recent));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&format!("recent: held, last change merged {recent}").as_str()),
        "{}",
        run.out
    );
    assert_eq!(
        sandbox.apm_calls(),
        ["update --yes o/r/skills/old own", "install", "audit"]
    );
}

#[test]
fn updates_a_skill_just_past_a_day() {
    let sandbox = Sandbox::new("just-past-a-day", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(&hours_ago(25)));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert_eq!(
        sandbox.apm_calls()[0],
        "update --yes o/r/skills/old o/r/skills/recent own"
    );
}

// The mise task runs it from the root of the checkout it ran in, whose apm.yml apm reads.
#[test]
fn reads_the_lockfile_and_runs_every_apm_call_in_the_directory_it_runs_in() {
    let sandbox = Sandbox::new("runs-where-it-is", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    let root = fs::canonicalize(&sandbox.root).expect("the sandbox resolves");
    let cwds = sandbox.log("apm.cwd");
    assert_eq!(cwds, vec![root.display().to_string(); 3]);
}

#[test]
fn passes_on_what_apm_prints_to_either_stream_on_stdout() {
    let sandbox = Sandbox::new("apm-output", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
        .arg("skills-update")
        .current_dir(&sandbox.root)
        .env("PATH", sandbox.search_path())
        .output()
        .expect("dotfiles-local starts");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "apm says update\napm warns update\napm says install\napm warns install\napm says audit\napm warns audit\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn asks_github_nothing_about_a_gist_and_updates_it() {
    let sandbox = Sandbox::new("gist", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        sandbox
            .gh_calls()
            .iter()
            .all(|path| !path.contains("me/abc")),
        "{:?}",
        sandbox.gh_calls()
    );
    assert!(sandbox.apm_calls()[0].ends_with(" own"));
}

#[test]
fn asks_about_a_whole_repository_by_its_commits_and_updates_it_by_name() {
    let sandbox = Sandbox::new(
        "whole-repository",
        "dependencies:\n- name: whole\n  host: github.com\n  repo_url: o/whole\n",
    );
    sandbox.github("o/whole", None, &Answer::Merged(OLD));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert_eq!(
        sandbox.gh_calls(),
        [
            "repos/o/whole/commits?per_page=1",
            "repos/o/whole/commits/sha-o-whole/pulls"
        ]
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes whole");
}

#[test]
fn holds_a_path_with_no_commit_and_the_reason_says_so() {
    let sandbox = Sandbox::new("no-commit", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::NoCommit);

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&"recent: held, no commit on this path"),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_commit_with_no_pull_request_with_a_reason_that_claims_no_more() {
    let sandbox = Sandbox::new("no-pull", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::NoPull);

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&format!("recent: held, {UNLISTED}").as_str()),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_commit_whose_only_pull_request_went_to_another_branch_with_the_same_reason() {
    let sandbox = Sandbox::new("other-branch", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::OtherBranch(OLD));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&format!("recent: held, {UNLISTED}").as_str()),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_skill_whose_request_fails_rather_than_updating_it() {
    let sandbox = Sandbox::new("request-fails", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Fail);

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&"recent: held, GitHub request failed (exit 1)"),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_skill_whose_merge_time_github_reports_in_a_form_it_cannot_read_saying_so() {
    let sandbox = Sandbox::new("not-a-timestamp", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged("yesterday"));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines().contains(
            &"recent: held, GitHub reported its last merge at \"yesterday\", not a timestamp"
        ),
        "{}",
        run.out
    );
}

#[test]
fn asks_about_and_updates_only_the_packages_named() {
    let sandbox = Sandbox::new("named", LOCK);
    sandbox.skill("recent", &Answer::Merged(OLD));

    let run = sandbox.run(&["o/r/skills/recent"]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert_eq!(
        sandbox.gh_calls(),
        [
            "repos/o/r/commits?path=skills/recent&per_page=1",
            "repos/o/r/commits/sha-skills-recent/pulls"
        ]
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/recent");
}

#[test]
fn refuses_a_name_the_lockfile_does_not_hold_asking_and_updating_nothing() {
    let sandbox = Sandbox::new("unknown-name", LOCK);

    let run = sandbox.run(&["o/r/skills/typo", "o/r/skills/old"]);

    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(1), "Not in apm.lock.yaml: o/r/skills/typo\n")
    );
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
    assert_eq!(sandbox.apm_calls(), Vec::<String>::new());
}

// A name matches one dependency, so the second time it is given there is none left for it.
#[test]
fn refuses_a_name_given_twice() {
    let sandbox = Sandbox::new("name-twice", LOCK);

    let run = sandbox.run(&["o/r/skills/old", "o/r/skills/old"]);

    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(1), "Not in apm.lock.yaml: o/r/skills/old\n")
    );
}

#[test]
fn runs_no_apm_when_every_chosen_skill_is_held() {
    let sandbox = Sandbox::new("all-held", LOCK);
    let recent = hours_ago(1);
    sandbox.skill("recent", &Answer::Merged(&recent));

    let run = sandbox.run(&["o/r/skills/recent"]);

    assert_eq!(
        (run.code, run.out),
        (
            Some(0),
            format!("recent: held, last change merged {recent}\nNothing to update.\n")
        )
    );
    assert_eq!(sandbox.apm_calls(), Vec::<String>::new());
}

#[test]
fn fails_with_apms_status_when_a_step_fails_running_none_after_it() {
    let sandbox = Sandbox::new("apm-fails", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));
    sandbox.apm_failing("install", 4);

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(4), "{}", run.out);
    assert!(
        run.out.ends_with("apm install failed (exit 4)\n"),
        "{}",
        run.out
    );
    assert_eq!(
        sandbox.apm_calls(),
        [
            "update --yes o/r/skills/old o/r/skills/recent own",
            "install"
        ]
    );
}

#[test]
fn fails_with_yqs_status_when_it_cannot_read_the_lockfile() {
    let sandbox = Sandbox::new("unreadable-lockfile", "dependencies: [\n");

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(
        run.out
            .ends_with("yq could not read apm.lock.yaml (exit 1)\n"),
        "{}",
        run.out
    );
}

#[test]
fn fails_at_a_lockfile_with_no_dependencies_list_naming_what_it_read() {
    let sandbox = Sandbox::new("no-dependencies", "lockfile_version: '1'\n");

    let run = sandbox.run(&[]);

    assert_eq!(
        (run.code, run.out.as_str()),
        (
            Some(1),
            "apm.lock.yaml has no dependencies list: {\"lockfile_version\":\"1\"}\n"
        )
    );
}

#[test]
fn fails_at_a_dependency_with_no_repository_before_asking_github_anything() {
    let sandbox = Sandbox::new(
        "no-repository",
        "dependencies:\n- name: lost\n  host: github.com\n  virtual_path: skills/lost\n",
    );

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(
        run.out
            .starts_with("a dependency in apm.lock.yaml names no repository: {"),
        "{}",
        run.out
    );
    assert!(run.out.contains("\"name\":\"lost\""), "{}", run.out);
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
}

#[test]
fn prints_its_usage_for_help_and_h_and_refuses_another_flag_with_exit_2() {
    let sandbox = Sandbox::new("usage", LOCK);

    for flag in ["--help", "-h"] {
        let run = sandbox.run(&[flag]);
        assert_eq!(run.code, Some(0), "{flag}");
        assert!(
            run.out
                .starts_with("Usage: mise run skills:update [package...]\n"),
            "{}",
            run.out
        );
    }
    let run = sandbox.run(&["--yes"]);
    assert_eq!(run.code, Some(2));
    assert!(run.out.starts_with("Usage: mise run skills:update"));
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
    assert_eq!(sandbox.apm_calls(), Vec::<String>::new());
}

// Only a first argument is read as a flag: --help with more after it is refused, and a
// flag after a name is taken as a name.
#[test]
fn reads_only_the_first_argument_as_a_flag() {
    let sandbox = Sandbox::new("flag-position", LOCK);

    let run = sandbox.run(&["--help", "o/r/skills/old"]);
    assert_eq!(run.code, Some(2), "{}", run.out);
    assert!(run.out.starts_with("Usage: mise run skills:update"));

    let run = sandbox.run(&["o/r/skills/old", "--yes"]);
    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(1), "Not in apm.lock.yaml: --yes\n")
    );
}
