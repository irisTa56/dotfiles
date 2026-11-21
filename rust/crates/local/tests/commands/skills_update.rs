//! `skills-update`: APM skills updated unless their last change upstream merged under a
//! day ago, run with a fake gh that answers `gh api <path>` with a file of GitHub's JSON
//! and refuses any other arguments, a fake apm that records how it was called, and yq
//! and jq failing wherever they are run.

use std::fs::{self, File};
use std::path::Path;
use std::process::{Output, Stdio};

use jiff::{SignedDuration, Timestamp};

use crate::support::{Sandbox, dotfiles_local};

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
struct Fixture {
    sandbox: Sandbox,
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

impl Fixture {
    /// A fresh sandbox named after `test`, holding `lock` as apm.lock.yaml, whose apm
    /// fails with `exit` when called with the subcommand `failing`, if any.
    fn new(test: &str, lock: &str) -> Self {
        let sandbox = Sandbox::new("skills-update", test, &["gh"]);
        sandbox.forbid(&["yq", "jq"]);
        fs::write(sandbox.root().join("apm.lock.yaml"), lock).expect("the lockfile is written");
        let fixture = Fixture { sandbox };
        fixture.apm_failing("", 0);
        // It answers a path with the file named after it, and fails where there is
        // none. It records each path it is asked.
        fixture.sandbox.fake(
            "gh",
            "dir=$(dirname \"$0\")/..\n\
             [ \"$#\" = 2 ] && [ \"$1\" = api ] || { echo \"gh got $*\" >&2; exit 2; }\n\
             echo \"$2\" >> \"$dir/gh.log\"\n\
             answer=\"$dir/gh/$(printf '%s' \"$2\" | tr '/?&=' '____')\"\n\
             [ -f \"$answer\" ] || exit 1\n\
             exec cat \"$answer\"\n",
        );
        fixture
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// Makes apm fail with `code` when its subcommand is `subcommand`.
    fn apm_failing(&self, subcommand: &str, code: u8) {
        self.sandbox.fake(
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
        fs::write(self.root().join("gh").join(file), json).expect("the answer is written");
    }

    /// Runs `skills-update` with `args` from the sandbox, finding the fakes first on PATH.
    fn run(&self, args: &[&str]) -> Run {
        let output: Output = dotfiles_local("skills-update")
            .args(args)
            .current_dir(self.root())
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

    /// Runs `skills-update` with `args` from the sandbox, with its `bin` alone on PATH,
    /// and returns its stdout and stderr apart.
    fn run_isolated(&self, args: &[&str]) -> Output {
        dotfiles_local("skills-update")
            .args(args)
            .current_dir(self.root())
            .env("PATH", self.sandbox.bin())
            .output()
            .expect("dotfiles-local starts")
    }

    /// The tests' own PATH, after the sandbox's `bin`.
    fn search_path(&self) -> std::ffi::OsString {
        self.sandbox.path_before_inherited()
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
        fs::read_to_string(self.root().join(name)).map_or_else(
            |_| Vec::new(),
            |log| log.lines().map(str::to_owned).collect(),
        )
    }
}

/// A list of one pull request, merged at `date` into `base` of a repository whose
/// default branch is `main`.
fn pulls(date: &str, base: &str) -> String {
    format!("[{}]", pull(date, base))
}

/// A pull request merged at `date` into `base` of a repository whose default branch is
/// `main`.
fn pull(date: &str, base: &str) -> String {
    format!(
        r#"{{"merged_at": "{date}", "base": {{"ref": "{base}", "repo": {{"default_branch": "main"}}}}}}"#
    )
}

fn hours_ago(hours: i64) -> String {
    (Timestamp::now() - SignedDuration::from_hours(hours)).to_string()
}

#[test]
fn updates_a_skill_whose_last_change_merged_over_a_day_ago_and_holds_a_newer_one() {
    let sandbox = Fixture::new("updates-and-holds", LOCK);
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
    let sandbox = Fixture::new("just-past-a-day", LOCK);
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
    let sandbox = Fixture::new("runs-where-it-is", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    let root = fs::canonicalize(sandbox.root()).expect("the sandbox resolves");
    let cwds = sandbox.log("apm.cwd");
    assert_eq!(cwds, vec![root.display().to_string(); 3]);
}

#[test]
fn passes_on_what_apm_prints_to_either_stream_on_stdout() {
    let sandbox = Fixture::new("apm-output", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::Merged(OLD));

    let output = dotfiles_local("skills-update")
        .current_dir(sandbox.root())
        .env("PATH", sandbox.search_path())
        .output()
        .expect("dotfiles-local starts");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "apm says update\napm warns update\napm says install\napm warns install\napm says audit\napm warns audit\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

// apm colors its output where it sees a terminal, so it must be given pipes whatever the
// binary's own stdout is, and what it prints through them must still reach that stdout.
#[test]
fn gives_apm_a_pipe_for_each_stream_even_when_its_own_stdout_is_a_file() {
    let fixture = Fixture::new("apm-pipes", LOCK);
    fixture.skill("old", &Answer::Merged(OLD));
    fixture.skill("recent", &Answer::Merged(OLD));
    fixture.sandbox.fake(
        "apm",
        "dir=$(dirname \"$0\")/..\n\
         for stream in stdout stderr; do\n\
         if test -p \"/dev/$stream\"; then kind=pipe; else kind=other; fi\n\
         echo \"$1 $stream $kind\" >> \"$dir/apm.streams\"\n\
         done\n\
         echo \"apm says $1\"\n\
         echo \"apm warns $1\" >&2\n",
    );
    let stdout_file = fixture.root().join("stdout.txt");

    let status = dotfiles_local("skills-update")
        .current_dir(fixture.root())
        .env("PATH", fixture.search_path())
        .stdout(File::create(&stdout_file).expect("the stdout file is created"))
        .stderr(Stdio::null())
        .status()
        .expect("dotfiles-local starts");

    assert!(status.success(), "{status}");
    let streams = fixture.log("apm.streams");
    let expected: Vec<String> = ["update", "install", "audit"]
        .iter()
        .flat_map(|step| [format!("{step} stdout pipe"), format!("{step} stderr pipe")])
        .collect();
    assert_eq!(
        streams, expected,
        "apm's stdout and stderr must each be a pipe"
    );
    assert_eq!(
        fs::read_to_string(&stdout_file).expect("the stdout file is read"),
        "apm says update\napm warns update\napm says install\napm warns install\napm says audit\napm warns audit\n"
    );
}

#[test]
fn asks_github_nothing_about_a_gist_and_updates_it() {
    let sandbox = Fixture::new("gist", LOCK);
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
    let sandbox = Fixture::new(
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

// The hold counts from the latest merge into the default branch, wherever GitHub lists
// it, and a later merge into another branch does not count.
#[test]
fn holds_by_the_latest_of_several_merges_into_the_default_branch() {
    let sandbox = Fixture::new("several-merges", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.skill("recent", &Answer::NoPull);
    let recent = hours_ago(2);
    sandbox.answer(
        "repos/o/r/commits/sha-skills-recent/pulls",
        &format!(
            "[{}, {}, {}, {}]",
            pull(OLD, "main"),
            pull(&recent, "main"),
            pull("2026-02-01T00:00:00Z", "main"),
            pull(&hours_ago(1), "dev")
        ),
    );

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .contains(&format!("recent: held, last change merged {recent}").as_str()),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_path_with_no_commit_and_the_reason_says_so() {
    let sandbox = Fixture::new("no-commit", LOCK);
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
    let sandbox = Fixture::new("no-pull", LOCK);
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
    let sandbox = Fixture::new("other-branch", LOCK);
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
    let sandbox = Fixture::new("request-fails", LOCK);
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

// gh prints what GitHub answered, which an error page can make other than JSON.
#[test]
fn holds_a_skill_whose_answer_from_github_is_not_json_saying_so() {
    let sandbox = Fixture::new("not-json", LOCK);
    sandbox.skill("old", &Answer::Merged(OLD));
    sandbox.answer("repos/o/r/commits?path=skills/recent&per_page=1", "<html>");

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(
        run.lines()
            .iter()
            .any(|line| line.starts_with("recent: held, GitHub's answer is not JSON: ")),
        "{}",
        run.out
    );
    assert_eq!(sandbox.apm_calls()[0], "update --yes o/r/skills/old own");
}

#[test]
fn holds_a_skill_whose_merge_time_github_reports_in_a_form_it_cannot_read_saying_so() {
    let sandbox = Fixture::new("not-a-timestamp", LOCK);
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
    let sandbox = Fixture::new("named", LOCK);
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
    let sandbox = Fixture::new("unknown-name", LOCK);

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
    let sandbox = Fixture::new("name-twice", LOCK);

    let run = sandbox.run(&["o/r/skills/old", "o/r/skills/old"]);

    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(1), "Not in apm.lock.yaml: o/r/skills/old\n")
    );
}

#[test]
fn runs_no_apm_when_every_chosen_skill_is_held() {
    let sandbox = Fixture::new("all-held", LOCK);
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
    let sandbox = Fixture::new("apm-fails", LOCK);
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
fn fails_with_exit_1_at_a_lockfile_that_is_not_yaml() {
    let sandbox = Fixture::new("unparsable-lockfile", "dependencies: [\n");

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(
        run.out.starts_with("cannot parse apm.lock.yaml: "),
        "{}",
        run.out
    );
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
}

#[test]
fn fails_at_a_lockfile_with_no_dependencies_list_naming_what_it_read() {
    let sandbox = Fixture::new("no-dependencies", "lockfile_version: '1'\n");

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
    let sandbox = Fixture::new(
        "no-repository",
        "dependencies:\n- name: lost\n  host: github.com\n  virtual_path: skills/lost\n",
    );

    let run = sandbox.run(&[]);

    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(
        run.out
            .starts_with("a dependency in apm.lock.yaml has no repository: {"),
        "{}",
        run.out
    );
    assert!(run.out.contains("\"name\":\"lost\""), "{}", run.out);
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
}

#[test]
fn prints_its_usage_for_help_and_h_and_refuses_another_flag_as_a_usage_error() {
    let sandbox = Fixture::new("usage", LOCK);

    for flag in ["--help", "-h"] {
        let run = sandbox.run(&[flag]);
        assert_eq!(run.code, Some(0), "{flag}");
        assert!(
            run.out
                .contains("Usage: dotfiles-local skills-update [PACKAGES]...\n"),
            "{}",
            run.out
        );
    }
    let run = sandbox.run(&["--yes"]);
    assert_eq!(run.code, Some(2));
    assert!(
        run.out.starts_with("error: unexpected argument '--yes'"),
        "{}",
        run.out
    );
    assert_eq!(sandbox.gh_calls(), Vec::<String>::new());
    assert_eq!(sandbox.apm_calls(), Vec::<String>::new());
}

// Only a first argument is read as a flag: a flag after a name is taken as a name.
#[test]
fn reads_only_the_first_argument_as_a_flag() {
    let sandbox = Fixture::new("flag-position", LOCK);

    let run = sandbox.run(&["--help", "o/r/skills/old"]);
    assert_eq!(run.code, Some(0), "{}", run.out);
    assert!(run.out.contains("Usage: dotfiles-local skills-update"));

    let run = sandbox.run(&["o/r/skills/old", "--yes"]);
    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(1), "Not in apm.lock.yaml: --yes\n")
    );
}

/// One dependency GitHub is asked about, and none that is not.
const WHOLE: &str =
    r#"{"dependencies": [{"name": "whole", "host": "github.com", "repo_url": "o/whole"}]}"#;

// A missing command ends the run rather than counting as an answer, such as a hold for
// every skill where gh is missing.
#[test]
fn fails_with_exit_1_naming_a_command_it_cannot_run() {
    for (missing, lock, message) in [
        ("gh", WHOLE, "cannot run gh: "),
        (
            "apm",
            r#"{"dependencies": [{"name": "own", "host": "gist.github.com"}]}"#,
            "apm update --yes own failed (",
        ),
    ] {
        let sandbox = Fixture::new(&format!("missing-{missing}"), lock);
        fs::remove_file(sandbox.root().join("bin").join(missing)).expect("the fake is removed");

        let output = sandbox.run_isolated(&[]);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{missing}: {stderr}");
        assert!(
            stderr
                .lines()
                .last()
                .is_some_and(|line| line.starts_with(message)),
            "{missing}: {stderr}"
        );
        assert_eq!(sandbox.apm_calls(), Vec::<String>::new(), "{missing}");
    }
}

#[test]
fn fails_with_exit_1_where_there_is_no_lockfile() {
    let fixture = Fixture::new("no-lockfile", LOCK);
    fs::remove_file(fixture.root().join("apm.lock.yaml")).expect("the lockfile is removed");

    let output = fixture.run_isolated(&[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.starts_with("cannot read apm.lock.yaml: No such file or directory"),
        "{stderr}"
    );
    assert_eq!(fixture.gh_calls(), Vec::<String>::new());
}

#[test]
fn passes_on_what_gh_prints_to_stderr() {
    let fixture = Fixture::new("tool-stderr", WHOLE);
    fixture
        .sandbox
        .fake("gh", "echo 'gh warns' >&2\necho '[]'\n");

    let output = fixture.run_isolated(&[]);

    assert_eq!(
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).as_ref(),
            String::from_utf8_lossy(&output.stderr).as_ref()
        ),
        (
            Some(0),
            "whole: held, no commit on this path\nNothing to update.\n",
            "gh warns\n"
        )
    );
}

#[test]
fn fails_with_128_and_the_signal_when_a_signal_ends_apm() {
    let fixture = Fixture::new("apm-signal", LOCK);
    fixture.skill("old", &Answer::Merged(OLD));
    fixture.skill("recent", &Answer::Merged(OLD));
    fixture.sandbox.fake("apm", "kill -9 $$\n");

    let run = fixture.run(&[]);

    assert_eq!(run.code, Some(137), "{}", run.out);
    assert!(
        run.out
            .ends_with("apm update --yes o/r/skills/old o/r/skills/recent own failed (exit 137)\n"),
        "{}",
        run.out
    );
}

#[test]
fn fails_at_a_dependency_with_no_name() {
    let sandbox = Fixture::new(
        "no-name",
        "dependencies:\n- host: gist.github.com\n  repo_url: me/abc\n",
    );

    let run = sandbox.run(&[]);

    assert_eq!(
        (run.code, run.out.as_str()),
        (
            Some(1),
            "a dependency in apm.lock.yaml has no name: {\"host\":\"gist.github.com\",\"repo_url\":\"me/abc\"}\n"
        )
    );
}

// apm reads nothing from whoever runs the command, so a prompt it might put ends at once
// rather than waiting on a terminal or pipe no one answers.
#[test]
fn gives_apm_no_input() {
    let fixture = Fixture::new("apm-stdin", LOCK);
    fixture.skill("old", &Answer::Merged(OLD));
    fixture.skill("recent", &Answer::Merged(OLD));
    let streams = fixture.root().join("apm.stdin");
    fixture.sandbox.fake(
        "apm",
        &format!(
            "if test -p /dev/stdin; then echo pipe; else echo other; fi >> '{}'\n",
            streams.display()
        ),
    );

    let output = dotfiles_local("skills-update")
        .current_dir(fixture.root())
        .env("PATH", fixture.search_path())
        .stdin(Stdio::piped())
        .output()
        .expect("dotfiles-local starts");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(streams).expect("apm recorded its input"),
        "other\nother\nother\n"
    );
}
