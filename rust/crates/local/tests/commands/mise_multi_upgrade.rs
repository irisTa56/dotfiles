//! `mise-multi-upgrade`: mise tools upgraded directory by directory, run with fakes for
//! mise and git that answer from files the test writes and record how they were called.

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// A directory of the test's own, holding the fakes, their answers and their logs,
/// removed when the test ends.
struct Sandbox {
    root: PathBuf,
}

/// What a run left: its exit status, its stdout and its stderr.
struct Run {
    code: Option<i32>,
    out: String,
    err: String,
}

impl Sandbox {
    /// A fresh sandbox named after `test`, whose mise tracks no config and whose git
    /// takes every directory for a checkout of its own.
    fn new(test: &str) -> Self {
        // Under this repository's `target/` rather than the system's temporary
        // directory, which the command leaves out.
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("mise-multi-upgrade")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        for dir in ["bin", "tmp", "answers"] {
            fs::create_dir_all(root.join(dir)).expect("the sandbox is created");
        }
        let sandbox = Sandbox { root };
        sandbox.fake_mise();
        sandbox.fake_git();
        sandbox
    }

    /// A git that takes a directory `worktree` lists for a linked worktree of the
    /// project beside it, or for no repository where that is `none`, and any other
    /// directory for a checkout of its own.
    fn fake_git(&self) {
        let root = self.root.display();
        self.fake(
            "git",
            &format!(
                "dir=$2
                 [ -f '{root}/worktrees' ] && while read -r worktree project; do
                 [ \"$worktree\" = \"$dir\" ] || continue
                 [ \"$project\" = none ] && {{ echo 'fatal: not a git repository' >&2; exit 128; }}
                 printf '%s/.git/worktrees/wt\\n%s/.git\\n' \"$project\" \"$project\"
                 exit 0
                 done < '{root}/worktrees'
                 printf '%s/.git\\n%s/.git\\n' \"$dir\" \"$dir\"
"
            ),
        );
    }

    /// Makes git take `dir` for a linked worktree of `project`, or for no repository
    /// where `project` is `none`.
    fn worktree(&self, dir: &str, project: &str) {
        append(&self.root.join("worktrees"), &format!("{dir} {project}\n"));
    }

    /// Makes mise track the config at `path`.
    fn track(&self, path: &str) {
        append(&self.root.join("tracked"), &format!("{path}\n"));
    }

    /// Makes mise track a mise.toml in `dir`, returned.
    fn project<'a>(&self, dir: &'a str) -> &'a str {
        self.track(&format!("{dir}/mise.toml"));
        dir
    }

    /// Makes `mise outdated` in `place` report `tools`, as (tool, current, latest),
    /// each requesting its current major: the tools `mise upgrade` there would upgrade.
    fn outdated_in(&self, place: Place<'_>, tools: &[(&str, &str, &str)]) {
        let entries = tools
            .iter()
            .map(|(tool, current, latest)| (*tool, entry(current, Some(latest), None)))
            .collect();
        self.outdated_json(place, false, &entries);
    }

    /// Makes `mise outdated --bump` in `place` report `tools`, as (tool, current, latest
    /// overall, request to bump to).
    fn bump_in(&self, place: Place<'_>, tools: &[(&str, &str, &str, Option<&str>)]) {
        let entries = tools
            .iter()
            .map(|(tool, current, latest, bump)| (*tool, entry(current, Some(latest), *bump)))
            .collect();
        self.outdated_json(place, true, &entries);
    }

    fn outdated_json(&self, place: Place<'_>, bump: bool, entries: &Vec<(&str, Value)>) {
        let object: serde_json::Map<String, Value> = entries
            .iter()
            .map(|(tool, entry)| ((*tool).to_owned(), entry.clone()))
            .collect();
        self.answer(
            &place.outdated(bump),
            &Value::Object(object).to_string(),
            "",
            0,
        );
    }

    fn fake(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
    }

    /// A mise that answers each argument list from the files `answer` writes for it,
    /// and otherwise as mise does where nothing is outdated, recording each call.
    fn fake_mise(&self) {
        let root = self.root.display();
        self.fake(
            "mise",
            &format!(
                "printf '%s\\n' \"$*\" >> '{root}/mise.log'\n\
                 answer='{root}/answers/'$(printf '%s' \"$*\" | /usr/bin/tr ' /' '_%')\n\
                 [ -f \"$answer.err\" ] && /bin/cat \"$answer.err\" >&2\n\
                 [ -f \"$answer.out\" ] && /bin/cat \"$answer.out\"\n\
                 [ -f \"$answer.code\" ] && exit \"$(/bin/cat \"$answer.code\")\"\n\
                 [ -f \"$answer.out\" ] && exit 0\n\
                 case \"$*\" in\n\
                 \"config ls --tracked-configs\") /bin/cat '{root}/tracked' 2>/dev/null ;;\n\
                 *\" outdated \"*) echo '{{}}' ;;\n\
                 \"ls-remote \"*) exit 1 ;;\n\
                 *upgrade*) echo \"mise says $*\"; echo \"mise warns $*\" >&2 ;;\n\
                 esac\n\
                 exit 0\n"
            ),
        );
    }

    /// Makes mise, called with `args`, print `out` and `err` and exit with `code`.
    fn answer(&self, args: &str, out: &str, err: &str, code: u8) {
        let key = args.replace(' ', "_").replace('/', "%");
        let answers = self.root.join("answers");
        for (extension, contents) in [("out", out), ("err", err), ("code", &code.to_string())] {
            fs::write(answers.join(format!("{key}.{extension}")), contents)
                .expect("the answer is written");
        }
    }

    /// Runs `mise-multi-upgrade` with `args` and `input` as stdin, the fakes first on
    /// PATH and `TMPDIR` naming the sandbox's own.
    fn run(&self, args: &[&str], input: &str) -> Run {
        self.run_with(args, input, &[("TMPDIR", &self.dir("tmp"))])
    }

    /// The path of `name` in the sandbox, as a string.
    fn dir(&self, name: &str) -> String {
        self.root.join(name).display().to_string()
    }

    /// Runs as `run` does, with the variables in `env` set, and `TEMP` and `TMP` unset
    /// unless they are among them.
    fn run_with(&self, args: &[&str], input: &str, env: &[(&str, &str)]) -> Run {
        let mut child = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("mise-multi-upgrade")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", self.path())
            .env_remove("TMPDIR")
            .env_remove("TEMP")
            .env_remove("TMP")
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("dotfiles-local starts");
        let mut stdin = child.stdin.take().expect("stdin is piped");
        // A run that stops reading early closes the pipe, which is no failure here.
        let _ = stdin.write_all(input.as_bytes());
        drop(stdin);
        let output = child.wait_with_output().expect("dotfiles-local ends");
        Run {
            code: output.status.code(),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Runs as `run` does with a stdout whose reader has already gone, as when `| head`
    /// stops reading: the exit status and stderr.
    fn run_unread(&self, args: &[&str], input: &str) -> (Option<i32>, String) {
        let (reader, writer) = std::io::pipe().expect("a pipe is made");
        drop(reader);
        let output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("mise-multi-upgrade")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", self.path())
            .env("TMPDIR", self.dir("tmp"))
            .stdin(Stdio::from(
                fs::File::open(self.input(input)).expect("the input opens"),
            ))
            .stdout(writer)
            .output()
            .expect("dotfiles-local runs");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Runs as `run` does with a directory as stdin, so that reading an answer fails
    /// with an I/O error: the exit status and stderr.
    fn run_unreadable_input(&self, args: &[&str]) -> (Option<i32>, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("mise-multi-upgrade")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", self.path())
            .env("TMPDIR", self.dir("tmp"))
            .stdin(Stdio::from(
                fs::File::open(&self.root).expect("the directory opens"),
            ))
            .output()
            .expect("dotfiles-local runs");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// A file holding `input`.
    fn input(&self, input: &str) -> PathBuf {
        let path = self.root.join("input");
        fs::write(&path, input).expect("the input is written");
        path
    }

    /// PATH with the fakes first.
    fn path(&self) -> std::ffi::OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        std::env::join_paths(
            std::iter::once(self.root.join("bin")).chain(std::env::split_paths(&inherited)),
        )
        .expect("PATH is joined")
    }

    /// The arguments of each mise call, joined by spaces.
    fn mise_calls(&self) -> Vec<String> {
        fs::read_to_string(self.root.join("mise.log")).map_or_else(
            |_| Vec::new(),
            |log| log.lines().map(str::to_owned).collect(),
        )
    }

    /// The mise calls that upgraded something, in the order they ran.
    fn upgrades(&self) -> Vec<String> {
        self.mise_calls()
            .into_iter()
            .filter(|call| call.split(' ').any(|arg| arg == "upgrade"))
            .collect()
    }
}

/// Where `mise outdated` is asked: the global config, or a project directory.
#[derive(Clone, Copy)]
enum Place<'a> {
    Global,
    Dir(&'a str),
}

impl Place<'_> {
    /// The arguments mise is called with to list what is outdated here.
    fn outdated(self, bump: bool) -> String {
        let base = match self {
            Place::Global => "-C / outdated --global".to_owned(),
            Place::Dir(dir) => format!("-C {dir} outdated --no-global"),
        };
        if bump {
            format!("{base} --bump --json")
        } else {
            format!("{base} --json")
        }
    }
}

/// An entry of `mise outdated --json`, with every key mise reports, requesting the
/// major of `current`.
fn entry(current: &str, latest: Option<&str>, bump: Option<&str>) -> Value {
    let requested = current.split('.').next().unwrap_or(current);
    json!({"requested": requested, "current": current, "latest": latest, "bump": bump})
}

fn append(path: &Path, text: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("the file is opened");
    file.write_all(text.as_bytes())
        .expect("the file is written");
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn prints_its_usage_for_help_and_h_asking_and_upgrading_nothing() {
    let sandbox = Sandbox::new("help");
    for flag in ["--help", "-h"] {
        let run = sandbox.run(&[flag], "y\n");
        assert_eq!(run.code, Some(0), "{flag}");
        assert!(
            run.out
                .starts_with("Usage: mise run mise:multi-upgrade [--dry-run [--bump]]"),
            "{flag}: {}",
            run.out
        );
        assert!(run.out.ends_with("for information. A `-` marks nothing to\nreport; a `?` marks a Released cell whose age could not be determined.\n\n"), "{flag}: {}", run.out);
    }
    assert_eq!(sandbox.mise_calls(), Vec::<String>::new());
}

const USAGE_START: &str = "Usage: mise run mise:multi-upgrade";

// Read as Elixir's `OptionParser` reads switches in strict mode, which the command was
// first written with.
#[test]
fn reads_its_switches_as_it_did_in_elixir() {
    let sandbox = Sandbox::new("switches");
    for args in [
        &["-nh"][..],
        &["-hn"],
        &["--dry-run=true", "--help=true"],
        &[
            "--include-worktrees",
            "--include-tmpdirs",
            "--bump",
            "-n",
            "-h",
        ],
        &[
            "--no-dry-run",
            "--no-bump",
            "--no-include-worktrees",
            "--no-include-tmpdirs",
            "-h",
        ],
        &["-h", "--"],
    ] {
        let run = sandbox.run(args, "");
        assert_eq!(run.code, Some(0), "{args:?}: {}", run.err);
        assert!(run.out.starts_with(USAGE_START), "{args:?}: {}", run.out);
    }
    for args in [
        &["--bogus"][..],
        &["-x"],
        &["-hx"],
        &["--dry_run", "-h"],
        &["--Dry-run", "-h"],
        &["--help=1"],
        &["--help=TRUE"],
        &["--no-n", "-h"],
        &["-h", "extra"],
        &["-h", "--", "--dry-run"],
        &["-h", "--", "--"],
        &["-h", "-"],
    ] {
        let run = sandbox.run(args, "");
        assert_eq!(
            (run.code, run.out.as_str()),
            (Some(2), ""),
            "{args:?}: {}",
            run.err
        );
        assert!(run.err.starts_with(USAGE_START), "{args:?}: {}", run.err);
        assert!(
            run.err.ends_with("determined.\n\n"),
            "{args:?}: {}",
            run.err
        );
    }
    assert_eq!(sandbox.mise_calls(), Vec::<String>::new());
}

#[test]
fn refuses_bump_without_dry_run_with_its_usage_and_exit_2() {
    let sandbox = Sandbox::new("bump-alone");
    for args in [
        &["--bump"][..],
        &["-n", "--bump", "--dry-run=false"],
        &["--bump", "--no-dry-run"],
    ] {
        let run = sandbox.run(args, "y\n");
        assert_eq!((run.code, run.out.as_str()), (Some(2), ""), "{args:?}");
        assert!(
            run.err
                .starts_with(&format!("--bump needs --dry-run\n\n{USAGE_START}")),
            "{args:?}: {}",
            run.err
        );
    }
    assert_eq!(sandbox.mise_calls(), Vec::<String>::new());
}

#[test]
fn asks_for_each_directory_then_upgrades_the_accepted_ones_after_the_last_answer() {
    let sandbox = Sandbox::new("asks");
    let a = sandbox.project("/work/a");
    let b = sandbox.project("/work/b");
    sandbox.outdated_in(Place::Dir(a), &[("jq", "1.6", "1.7")]);
    sandbox.outdated_in(Place::Dir(b), &[("yq", "4.1", "4.2")]);

    let run = sandbox.run(&[], "y\nn\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        run.out,
        "\n\
         /work/a\n\
         Tool  Requested  Current  Released  Latest  Released\n\
         ----  ---------  -------  --------  ------  --------\n\
         jq    1          1.6      ?         1.7     ?\n\
         Upgrade /work/a? [y/N] \n\
         /work/b\n\
         Tool  Requested  Current  Released  Latest  Released\n\
         ----  ---------  -------  --------  ------  --------\n\
         yq    4          4.1      ?         4.2     ?\n\
         Upgrade /work/b? [y/N] \n\
         ==> mise -C /work/a upgrade --no-global\n\
         mise says -C /work/a upgrade --no-global\n"
    );
    assert_eq!(run.err, "mise warns -C /work/a upgrade --no-global\n");
    assert_eq!(sandbox.upgrades(), ["-C /work/a upgrade --no-global"]);
}

/// A project with one tool to upgrade, plus a linked worktree of it that mise also
/// tracks, both with jq outdated in their own view.
fn with_worktree(sandbox: &Sandbox) -> (&'static str, &'static str) {
    let dir = sandbox.project("/work/project");
    let worktree = sandbox.project("/work/project/.claude/worktrees/wt");
    sandbox.worktree(worktree, dir);
    for place in [dir, worktree] {
        sandbox.outdated_in(Place::Dir(place), &[("jq", "1.6", "1.7")]);
    }
    (dir, worktree)
}

#[test]
fn leaves_out_a_linked_worktree() {
    let sandbox = Sandbox::new("worktree");
    let (dir, worktree) = with_worktree(&sandbox);

    let run = sandbox.run(&[], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(!run.out.contains(worktree), "{}", run.out);
    assert_eq!(
        sandbox.upgrades(),
        [format!("-C {dir} upgrade --no-global")]
    );
}

#[test]
fn takes_in_a_linked_worktree_with_include_worktrees() {
    let sandbox = Sandbox::new("include-worktrees");
    let (dir, worktree) = with_worktree(&sandbox);

    let run = sandbox.run(&["--include-worktrees"], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        sandbox.upgrades(),
        [
            format!("-C {dir} upgrade --no-global"),
            format!("-C {worktree} upgrade --no-global")
        ]
    );
}

#[test]
fn keeps_a_directory_git_does_not_know() {
    let sandbox = Sandbox::new("no-repository");
    let dir = sandbox.project("/work/plain");
    sandbox.worktree(dir, "none");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    let run = sandbox.run(&[], "y\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        sandbox.upgrades(),
        [format!("-C {dir} upgrade --no-global")]
    );
}

#[test]
fn upgrades_every_config_directory_form_from_its_project_root() {
    let sandbox = Sandbox::new("config-forms");
    let forms = [
        ".config/mise/conf.d/a.toml",
        ".mise/conf.d/a.toml",
        "mise/conf.d/a.toml",
        ".config/mise/config.toml",
        ".config/mise/mise.toml",
        ".config/mise.toml",
        ".mise/config.toml",
        "mise/config.toml",
    ];
    let dirs: Vec<String> = forms
        .iter()
        .enumerate()
        .map(|(i, form)| {
            let dir = format!("/work/p{i}");
            sandbox.track(&format!("{dir}/{form}"));
            sandbox.outdated_in(Place::Dir(&dir), &[("jq", "1.6", "1.7")]);
            dir
        })
        .collect();

    let run = sandbox.run(&[], &"y\n".repeat(forms.len()));

    assert_eq!(run.code, Some(0), "{}", run.err);
    let expected: Vec<String> = dirs
        .iter()
        .map(|dir| format!("-C {dir} upgrade --no-global"))
        .collect();
    assert_eq!(sandbox.upgrades(), expected);
}

#[test]
fn keeps_a_project_named_like_a_config_directory_under_its_own_path() {
    let sandbox = Sandbox::new("config-named");
    let dirs = ["/work/pmise/mise", "/work/p.mise/.mise"];
    for dir in dirs {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }

    let run = sandbox.run(&[], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    let mut expected: Vec<String> = dirs
        .iter()
        .map(|dir| format!("-C {dir} upgrade --no-global"))
        .collect();
    expected.sort();
    assert_eq!(sandbox.upgrades(), expected);
}

#[test]
fn asks_once_and_upgrades_once_for_a_directory_holding_two_configs() {
    let sandbox = Sandbox::new("two-configs");
    let dir = sandbox.project("/work/project");
    sandbox.track("/work/project/mise.local.toml");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    let run = sandbox.run(&[], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(run.out.matches("Upgrade /work/project?").count(), 1);
    assert_eq!(sandbox.upgrades(), ["-C /work/project upgrade --no-global"]);
}

/// Tracks a project with one tool to upgrade, returned, plus a config under each of
/// `scratch`, as under a temporary directory, with the same.
fn with_scratch(sandbox: &Sandbox, scratch: &[&str]) -> &'static str {
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    for dir in scratch {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }
    dir
}

#[test]
fn leaves_out_a_config_under_tmpdir_by_the_name_it_resolves_to_too() {
    let sandbox = Sandbox::new("tmpdir");
    let real = sandbox.dir("real");
    let link = sandbox.dir("link");
    fs::create_dir(&real).expect("the temporary directory is created");
    std::os::unix::fs::symlink(&real, &link).expect("the link is created");
    let named = format!("{link}/named");
    let resolved = format!("{real}/resolved");
    let dir = with_scratch(&sandbox, &[&named, &resolved]);

    // As a shell variable may name it, with a trailing slash and a step back.
    let run = sandbox.run_with(&[], "y\ny\ny\n", &[("TMPDIR", &format!("{link}/../link/"))]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        !run.out.contains(&link) && !run.out.contains(&real),
        "{}",
        run.out
    );
    assert_eq!(
        sandbox.upgrades(),
        [format!("-C {dir} upgrade --no-global")]
    );
}

// TMPDIR is passed over where it names no directory, or one that cannot be written,
// as Elixir's `System.tmp_dir/0` passes it over, and TEMP, then TMP, are taken in turn.
#[test]
fn takes_temp_or_tmp_where_tmpdir_names_no_directory_it_can_write() {
    let sandbox = Sandbox::new("temp");
    let temp = sandbox.dir("temp");
    let locked = sandbox.dir("locked");
    for dir in [&temp, &locked] {
        fs::create_dir(dir).expect("the temporary directory is created");
    }
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555))
        .expect("the directory is locked");
    let dir = with_scratch(&sandbox, &[&format!("{temp}/scratch")]);
    for env in [
        &[("TMPDIR", sandbox.dir("missing").as_str()), ("TEMP", &temp)][..],
        &[("TMPDIR", ""), ("TMP", &temp)],
        &[("TMPDIR", &locked), ("TEMP", &locked), ("TMP", &temp)],
    ] {
        fs::remove_file(sandbox.root.join("mise.log")).ok();

        let run = sandbox.run_with(&[], "y\ny\n", env);

        assert_eq!(run.code, Some(0), "{env:?}: {}", run.err);
        assert_eq!(
            sandbox.upgrades(),
            [format!("-C {dir} upgrade --no-global")],
            "{env:?}"
        );
    }
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))
        .expect("the directory is unlocked");
}

#[test]
fn leaves_out_a_config_under_tmp_whatever_tmpdir_says_by_the_name_it_resolves_to_too() {
    let sandbox = Sandbox::new("tmp");
    let resolved = fs::canonicalize("/tmp").expect("/tmp resolves");
    let resolved = format!("{}/resolved", resolved.display());
    let dir = with_scratch(&sandbox, &["/tmp/named", &resolved]);
    // A directory whose name only starts with the temporary one's is not under it.
    let sibling = sandbox.project("/tmp-other/project");
    sandbox.outdated_in(Place::Dir(sibling), &[("jq", "1.6", "1.7")]);

    let run = sandbox.run(&[], "y\ny\ny\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        !run.out.contains("/named") && !run.out.contains("/resolved"),
        "{}",
        run.out
    );
    assert_eq!(
        sandbox.upgrades(),
        [
            "-C /tmp-other/project upgrade --no-global".to_owned(),
            format!("-C {dir} upgrade --no-global")
        ]
    );
}

#[test]
fn takes_in_a_config_under_a_temporary_directory_with_include_tmpdirs() {
    let sandbox = Sandbox::new("include-tmpdirs");
    let scratch = format!("{}/scratch", sandbox.dir("tmp"));
    let dir = with_scratch(&sandbox, &["/tmp/scratch", &scratch]);

    let run = sandbox.run(&["--include-tmpdirs"], "y\ny\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    let mut expected = vec![
        format!("-C {dir} upgrade --no-global"),
        "-C /tmp/scratch upgrade --no-global".to_owned(),
        format!("-C {scratch} upgrade --no-global"),
    ];
    expected.sort();
    let mut upgrades = sandbox.upgrades();
    upgrades.sort();
    assert_eq!(upgrades, expected);
}

#[test]
fn names_a_directory_under_the_home_directory_from_a_tilde() {
    let sandbox = Sandbox::new("home");
    let dir = sandbox.project("/work/home/work/project");
    sandbox.project("/work/homely");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    sandbox.outdated_in(Place::Dir("/work/homely"), &[("jq", "1.6", "1.7")]);

    let run = sandbox.run_with(&[], "n\nn\n", &[("HOME", "/work/home")]);

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(run.out.contains("\n~/work/project\n"), "{}", run.out);
    assert!(
        run.out.contains("Upgrade ~/work/project? [y/N] "),
        "{}",
        run.out
    );
    assert!(
        run.out.contains("Upgrade /work/homely? [y/N] "),
        "{}",
        run.out
    );
}

/// An ISO 8601 time `days` whole days and an hour ago, in UTC with an offset.
fn days_ago(days: i64) -> String {
    let time = jiff::Timestamp::now() - jiff::SignedDuration::from_hours(days * 24 + 1);
    time.to_string()
}

#[test]
fn shows_how_long_ago_each_version_was_released() {
    let sandbox = Sandbox::new("ages");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(
        Place::Dir(dir),
        &[
            ("gh", "2.1", "2.2"),
            ("jq", "1.6", "1.7"),
            ("yq", "4.1", "4.2"),
            ("uv", "0.1", "0.2"),
        ],
    );
    let naive = days_ago(5).trim_end_matches('Z').to_owned();
    let offset = (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(2 * 24 + 1))
        .to_zoned_datetime(jiff::tz::TimeZone::fixed(jiff::tz::offset(9)))
        .strftime("%Y-%m-%dT%H:%M:%S%:z")
        .to_string();
    let remote = |tool: &str, versions: Value| {
        sandbox.answer(
            &format!("ls-remote {tool} --json"),
            &versions.to_string(),
            "",
            0,
        );
    };
    remote(
        "gh",
        json!([
            {"version": "2.1", "created_at": days_ago(1)},
            {"version": "2.2", "created_at": ""},
        ]),
    );
    remote(
        "jq",
        json!([
            {"version": "1.6", "created_at": days_ago(30)},
            {"version": "1.7", "created_at": (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(23)).to_string()},
        ]),
    );
    remote(
        "yq",
        json!([
            {"version": "4.1", "created_at": naive},
            {"version": "4.2", "created_at": offset},
        ]),
    );
    remote(
        "uv",
        json!([
            {"version": "0.1", "created_at": "last week"},
            {"version": "0.2"},
        ]),
    );

    let run = sandbox.run(&["--dry-run"], "");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        run.out,
        "\n\
         /work/project\n\
         Tool  Requested  Current  Released  Latest  Released\n\
         ----  ---------  -------  --------  ------  --------\n\
         gh    2          2.1      1d ago    2.2     -\n\
         jq    1          1.6      30d ago   1.7     today\n\
         uv    0          0.1      ?         0.2     -\n\
         yq    4          4.1      5d ago    4.2     2d ago\n"
    );
}

#[test]
fn reads_a_list_it_cannot_read_as_release_ages_unknown() {
    let sandbox = Sandbox::new("unreadable-lists");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(
        Place::Dir(dir),
        &[
            ("gh", "2.1", "2.2"),
            ("jq", "1.6", "1.7"),
            ("uv", "0.1", "0.2"),
            ("yq", "4.1", "4.2"),
            ("zx", "8.1", "8.2"),
        ],
    );
    sandbox.answer("ls-remote gh --json", r#"["2.1", "2.2"]"#, "", 0);
    sandbox.answer("ls-remote jq --json", "", "mise ERROR no such tool\n", 1);
    sandbox.answer("ls-remote uv --json", "null", "", 0);
    sandbox.answer("ls-remote yq --json", "not json", "", 0);
    sandbox.answer(
        "ls-remote zx --json",
        r#"[{"version": "8.2", "created_at": "2026-01-01T00:00:00Z"}, {"created_at": "2026-01-02T00:00:00Z"}]"#,
        "",
        0,
    );

    let run = sandbox.run(&["--dry-run"], "");

    assert_eq!(run.code, Some(0), "{}", run.err);
    for tool in ["gh", "jq", "uv", "yq", "zx"] {
        let line = run
            .out
            .lines()
            .find(|line| line.starts_with(&format!("{tool} ")))
            .unwrap_or_else(|| panic!("{tool} has a row: {}", run.out));
        let cells: Vec<&str> = line.split_whitespace().collect();
        assert_eq!((cells[3], cells[5]), ("?", "?"), "{line}");
    }
    assert_eq!(run.err, "mise ERROR no such tool\n");
}

#[test]
fn upgrades_the_global_config_apart_with_global() {
    let sandbox = Sandbox::new("global");
    let dotfiles = sandbox.project("/work/dotfiles");
    sandbox.track("/work/dotfiles/.config/mise/config.toml");
    sandbox.outdated_in(Place::Global, &[("jq", "1.6", "1.7")]);
    sandbox.outdated_in(Place::Dir(dotfiles), &[("yq", "4.1", "4.2")]);

    let run = sandbox.run(&[], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(run.out.starts_with("\nglobal\nTool "), "{}", run.out);
    assert!(run.out.contains("Upgrade global? [y/N] "), "{}", run.out);
    assert!(
        run.out.contains("\n==> mise upgrade --global\n"),
        "{}",
        run.out
    );
    assert!(!run.out.contains("/work/dotfiles/.config"), "{}", run.out);
    assert_eq!(
        sandbox.upgrades(),
        ["upgrade --global", "-C /work/dotfiles upgrade --no-global"]
    );
}

#[test]
fn goes_on_past_a_failed_upgrade_and_exits_1() {
    let sandbox = Sandbox::new("upgrade-fails");
    for dir in ["/work/a", "/work/b", "/work/c"] {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }
    sandbox.answer("-C /work/a upgrade --no-global", "", "mise ERROR a\n", 3);
    sandbox.answer("-C /work/c upgrade --no-global", "", "mise ERROR c\n", 1);

    let run = sandbox.run(&[], "y\ny\ny\n");

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert_eq!(
        run.err,
        "mise ERROR a\nmise warns -C /work/b upgrade --no-global\nmise ERROR c\nUpgrade failed in: /work/a, /work/c\n"
    );
    assert_eq!(
        sandbox.upgrades(),
        [
            "-C /work/a upgrade --no-global",
            "-C /work/b upgrade --no-global",
            "-C /work/c upgrade --no-global"
        ]
    );
}

#[test]
fn goes_on_past_places_mise_cannot_list_and_names_them_last_in_order() {
    let sandbox = Sandbox::new("unlisted");
    for dir in ["/work/a", "/work/b", "/work/c", "/work/d"] {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }
    for place in [Place::Dir("/work/d"), Place::Global, Place::Dir("/work/b")] {
        sandbox.answer(&place.outdated(false), "", "mise ERROR not trusted\n", 1);
    }

    let run = sandbox.run(&[], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        sandbox.upgrades(),
        [
            "-C /work/a upgrade --no-global",
            "-C /work/c upgrade --no-global"
        ]
    );
    assert!(
        run.err
            .ends_with("\nSkipped, as mise could not list them: global, /work/b, /work/d\n"),
        "{}",
        run.err
    );
    assert_eq!(run.err.matches("mise ERROR not trusted\n").count(), 3);
}

#[test]
fn skips_a_place_whose_bump_listing_fails() {
    let sandbox = Sandbox::new("unlisted-bump");
    for dir in ["/work/a", "/work/b"] {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }
    sandbox.answer(&Place::Dir("/work/b").outdated(true), "", "", 1);

    let run = sandbox.run(&["-n", "--bump"], "");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(!run.out.contains("/work/b"), "{}", run.out);
    assert_eq!(run.err, "\nSkipped, as mise could not list them: /work/b\n");
}

#[test]
fn shows_every_directory_at_once_with_dry_run_asking_and_upgrading_nothing() {
    let sandbox = Sandbox::new("dry-run");
    let a = sandbox.project("/work/a");
    let b = sandbox.project("/work/b");
    sandbox.outdated_in(Place::Dir(a), &[("jq", "1.6", "1.7")]);
    sandbox.bump_in(Place::Dir(b), &[("yq", "3.4", "4.2", Some("4"))]);

    let run = sandbox.run(&["--dry-run", "--bump"], "y\ny\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        run.out,
        "\n\
         /work/a\n\
         Tool  Requested  Current  Released  Latest  Released  Bump Latest  Released\n\
         ----  ---------  -------  --------  ------  --------  -----------  --------\n\
         jq    1          1.6      ?         1.7     ?         -            -\n\
         \n\
         /work/b\n\
         Tool  Requested  Current  Released  Latest  Released  Bump Latest  Released\n\
         ----  ---------  -------  --------  ------  --------  -----------  --------\n\
         yq    3          3.4      ?         3.4     ?         4.2          ?\n"
    );
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

#[test]
fn shows_a_tool_in_both_answers_once_with_a_bump_only_where_one_is_named() {
    let sandbox = Sandbox::new("both-answers");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(
        Place::Dir(dir),
        &[("node", "24.1.0", "24.2.0"), ("uv", "0.1.0", "0.2.0")],
    );
    sandbox.bump_in(
        Place::Dir(dir),
        &[
            ("node", "24.1.0", "26.0.0", Some("26")),
            ("uv", "0.1.0", "0.2.0", None),
        ],
    );

    let run = sandbox.run(&["--dry-run", "--bump"], "");

    assert_eq!(run.code, Some(0), "{}", run.err);
    let rows: Vec<&str> = run.out.lines().skip(4).collect();
    assert_eq!(
        rows,
        [
            "node  24         24.1.0   ?         24.2.0  ?         26.0.0       ?",
            "uv    0          0.1.0    ?         0.2.0   ?         -            -"
        ]
    );
}

#[test]
fn leaves_out_every_bump_and_the_tools_only_a_bump_would_reach_without_bump() {
    let sandbox = Sandbox::new("no-bump");
    let a = sandbox.project("/work/a");
    let b = sandbox.project("/work/b");
    sandbox.outdated_in(Place::Dir(a), &[("node", "24.1.0", "24.2.0")]);
    sandbox.bump_in(Place::Dir(a), &[("node", "24.1.0", "26.0.0", Some("26"))]);
    sandbox.bump_in(Place::Dir(b), &[("yq", "3.4", "4.2", Some("4"))]);

    let run = sandbox.run(&["--dry-run"], "");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        !run.out.contains("Bump") && !run.out.contains("26.0.0"),
        "{}",
        run.out
    );
    assert!(!run.out.contains(b), "{}", run.out);
    assert!(
        sandbox
            .mise_calls()
            .iter()
            .all(|call| !call.contains("--bump")),
        "{:?}",
        sandbox.mise_calls()
    );
}

#[test]
fn asks_nothing_for_a_directory_newer_only_past_its_requests() {
    let sandbox = Sandbox::new("bump-only");
    let dir = sandbox.project("/work/project");
    sandbox.bump_in(Place::Dir(dir), &[("yq", "3.4", "4.2", Some("4"))]);

    let run = sandbox.run(&[], "y\n");

    assert_eq!(
        (run.code, run.out.as_str()),
        (
            Some(0),
            "Nothing to upgrade within the requested versions.\n"
        )
    );
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

#[test]
fn says_all_tools_are_up_to_date_when_dry_run_has_no_table_to_show() {
    let sandbox = Sandbox::new("up-to-date");
    sandbox.project("/work/project");

    let run = sandbox.run(&["--dry-run"], "");

    assert_eq!(
        (run.code, run.out.as_str()),
        (Some(0), "All tools are up to date.\n")
    );
    assert!(
        sandbox
            .mise_calls()
            .iter()
            .all(|call| !call.starts_with("ls-remote")),
        "{:?}",
        sandbox.mise_calls()
    );
}

#[test]
fn answers_no_to_an_empty_line_and_the_rest_at_end_of_input_and_asks_again_otherwise() {
    let sandbox = Sandbox::new("answers");
    let dirs = ["/work/a", "/work/b", "/work/c", "/work/d", "/work/e"];
    for dir in dirs {
        sandbox.project(dir);
        sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    }

    // The last answer has no line end, and still counts.
    let run = sandbox.run(&[], "\nmaybe\nY\n  YES \nn");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert_eq!(
        sandbox.upgrades(),
        [
            "-C /work/b upgrade --no-global",
            "-C /work/c upgrade --no-global"
        ]
    );
    assert_eq!(run.out.matches("Upgrade /work/a? [y/N] ").count(), 1);
    assert!(
        run.out
            .contains("Upgrade /work/b? [y/N] Upgrade /work/b? [y/N] \n/work/c\n"),
        "{}",
        run.out
    );
    assert!(
        run.out
            .contains("Upgrade /work/e? [y/N] \n\n==> mise -C /work/b"),
        "{}",
        run.out
    );
}

#[test]
fn says_nothing_was_upgraded_when_every_directory_is_answered_no() {
    let sandbox = Sandbox::new("all-no");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    let run = sandbox.run(&[], "n\n");

    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        run.out
            .ends_with("Upgrade /work/project? [y/N] Nothing upgraded.\n"),
        "{}",
        run.out
    );
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

#[test]
fn stops_at_a_mise_outdated_entry_that_lacks_a_key_mise_always_reports() {
    let sandbox = Sandbox::new("broken-entry");
    let dir = sandbox.project("/work/project");
    for key in ["requested", "current", "latest"] {
        let mut entry = entry("1.6", Some("1.7"), None);
        entry
            .as_object_mut()
            .expect("an entry is an object")
            .remove(key);
        sandbox.answer(
            &Place::Dir(dir).outdated(false),
            &json!({"jq": entry}).to_string(),
            "",
            0,
        );

        let run = sandbox.run(&["--dry-run"], "");

        assert_eq!(
            (run.code, run.out.as_str(), run.err.as_str()),
            (
                Some(1),
                "",
                "mise outdated in /work/project gave an entry for jq that lacks a version it always reports\n"
            ),
            "{key}"
        );
    }
}

#[test]
fn stops_at_a_mise_outdated_bump_entry_that_names_a_bump_but_no_latest_version() {
    let sandbox = Sandbox::new("broken-bump");
    let dir = sandbox.project("/work/project");
    sandbox.answer(
        &Place::Dir(dir).outdated(true),
        r#"{"jq": {"requested": "1", "current": "1.6", "bump": "2"}}"#,
        "",
        0,
    );

    let run = sandbox.run(&["--dry-run", "--bump"], "");

    assert_eq!(run.code, Some(1), "{}", run.err);
    assert!(run.err.contains("entry for jq"), "{}", run.err);
}

#[test]
fn stops_at_mise_outdated_output_that_is_not_a_json_object() {
    let sandbox = Sandbox::new("outdated-not-json");
    let dir = sandbox.project("/work/project");
    for out in ["not json", "[]"] {
        sandbox.answer(&Place::Dir(dir).outdated(false), out, "", 0);

        let run = sandbox.run(&["--dry-run"], "");

        assert_eq!((run.code, run.out.as_str()), (Some(1), ""), "{out}");
        assert!(
            run.err.starts_with(
                "mise -C /work/project outdated --no-global --json gave no JSON object: "
            ),
            "{out}: {}",
            run.err
        );
    }
}

#[test]
fn fails_rather_than_reading_as_up_to_date_when_mise_cannot_list_its_configs() {
    let sandbox = Sandbox::new("no-configs");
    sandbox.outdated_in(Place::Global, &[("jq", "1.6", "1.7")]);
    sandbox.answer("config ls --tracked-configs", "", "mise ERROR broken\n", 1);

    let run = sandbox.run(&[], "y\n");

    assert_eq!(
        (run.code, run.out.as_str(), run.err.as_str()),
        (
            Some(1),
            "",
            "mise ERROR broken\nmise config ls --tracked-configs failed\n"
        )
    );
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

#[test]
fn fails_when_there_is_no_mise_or_no_git() {
    let sandbox = Sandbox::new("missing");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);
    let bin = sandbox.dir("bin");
    fs::remove_file(format!("{bin}/git")).expect("git is removed");

    let run = sandbox.run_with(&["--dry-run"], "", &[("PATH", &bin)]);

    assert_eq!((run.code, run.out.as_str()), (Some(1), ""));
    assert!(run.err.starts_with("cannot run git: "), "{}", run.err);

    fs::remove_file(format!("{bin}/mise")).expect("mise is removed");

    let run = sandbox.run_with(&["--dry-run"], "", &[("PATH", &bin)]);

    assert_eq!((run.code, run.out.as_str()), (Some(1), ""));
    assert!(run.err.starts_with("cannot run mise: "), "{}", run.err);
}

// The pipe a report is written to closes when its reader stops early, as with `| head`,
// which ends a report but fails a run that would upgrade what nobody saw asked about.
#[test]
fn ends_a_dry_run_quietly_when_its_reader_stops_early_and_fails_a_run_that_asks() {
    let sandbox = Sandbox::new("unread");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    assert_eq!(
        sandbox.run_unread(&["--dry-run"], ""),
        (Some(0), String::new())
    );

    let (code, err) = sandbox.run_unread(&[], "y\n");

    assert_eq!(code, Some(1));
    assert!(err.starts_with("cannot write the output: "), "{err}");
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

// An answer that cannot be read fails the run as a failure to read, not to write.
#[test]
fn fails_a_run_whose_answer_cannot_be_read_naming_the_read_and_upgrades_nothing() {
    let sandbox = Sandbox::new("unreadable");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    let (code, err) = sandbox.run_unreadable_input(&[]);

    assert_eq!(code, Some(1), "{err}");
    assert!(!err.contains("cannot write"), "{err}");
    assert!(err.contains("read"), "{err}");
    assert_eq!(sandbox.upgrades(), Vec::<String>::new());
}

// Any other failure to write fails a report too; `/dev/full` fails every write with no
// space left, and only Linux has it.
#[cfg(target_os = "linux")]
#[test]
fn fails_a_dry_run_whose_output_cannot_be_written() {
    let sandbox = Sandbox::new("full");
    let dir = sandbox.project("/work/project");
    sandbox.outdated_in(Place::Dir(dir), &[("jq", "1.6", "1.7")]);

    let output = Command::new(env!("CARGO_BIN_EXE_dotfiles-local"))
        .args(["mise-multi-upgrade", "--dry-run"])
        .current_dir(&sandbox.root)
        .env("PATH", sandbox.path())
        .env("TMPDIR", sandbox.dir("tmp"))
        .stdin(Stdio::null())
        .stdout(fs::File::create("/dev/full").expect("/dev/full opens"))
        .output()
        .expect("dotfiles-local runs");

    assert_eq!(output.status.code(), Some(1));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.starts_with("cannot write the output: "), "{err}");
}

/// A throwaway home for the real mise and git, under the system's temporary directory
/// and so outside this repository, where no config of the repository is an ancestor of
/// any directory mise runs in. mise is also stopped from looking above it, and holds a
/// local asdf plugin, `dummy`, that lists versions 1.0.0, 1.1.0 and 2.0.0 and installs
/// any of them without the network.
struct RealMise {
    dir: PathBuf,
}

impl RealMise {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "dotfiles-local-mise-multi-upgrade-real-{}-{test}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the directory is created");
        // mise reports a config by its resolved path, `/var` being a link on macOS.
        let dir = fs::canonicalize(&dir).expect("the directory resolves");
        let real = RealMise { dir };
        let plugin = real.dir.join("mise/plugin/bin");
        fs::create_dir_all(&plugin).expect("the plugin's directory is created");
        for (name, body) in [
            ("list-all", "echo \"1.0.0 1.1.0 2.0.0\"\n"),
            ("download", "exit 0\n"),
            (
                "install",
                "mkdir -p \"$ASDF_INSTALL_PATH/bin\"\n\
                 printf '#!/bin/sh\\necho %s\\n' \"$ASDF_INSTALL_VERSION\" > \"$ASDF_INSTALL_PATH/bin/dummy\"\n\
                 chmod +x \"$ASDF_INSTALL_PATH/bin/dummy\"\n",
            ),
        ] {
            let path = plugin.join(name);
            fs::write(&path, format!("#!/bin/sh\n{body}")).expect("the plugin is written");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("the plugin is made executable");
        }
        let plugin = real.dir.join("mise/plugin").display().to_string();
        real.command("mise", &["plugins", "link", "dummy", &plugin], &real.dir);
        real
    }

    /// `program` with the environment every call here runs in: a shell mise is
    /// activated in sets `__MISE_SESSION` and the like, from which mise runs the hooks of
    /// the configs that shell loaded, and a git hook running the tests sets `GIT_DIR`
    /// and the like for its own repository.
    fn env(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        for (name, _) in std::env::vars_os() {
            let name = name.to_string_lossy();
            if ["MISE_", "__MISE_", "GIT_"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
            {
                command.env_remove(name.as_ref());
            }
        }
        let mise = self.dir.join("mise");
        command
            .current_dir(&self.dir)
            .env("GIT_CEILING_DIRECTORIES", &self.dir)
            .env("MISE_DATA_DIR", mise.join("data"))
            .env("MISE_CONFIG_DIR", mise.join("config"))
            .env("MISE_STATE_DIR", mise.join("state"))
            .env("MISE_CACHE_DIR", mise.join("cache"))
            .env("MISE_GLOBAL_CONFIG_FILE", mise.join("config/config.toml"))
            .env("MISE_SYSTEM_CONFIG_FILE", mise.join("system.toml"))
            .env("MISE_TRUSTED_CONFIG_PATHS", &self.dir)
            .env("MISE_CEILING_PATHS", &self.dir)
            .env("MISE_NO_HOOKS", "1");
        command
    }

    /// Runs `program` with `args` in `cwd`, failing the test where it fails; its stdout.
    fn command(&self, program: &str, args: &[&str], cwd: &Path) -> String {
        let output = self
            .env(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap_or_else(|error| panic!("{program} runs: {error}"));
        assert!(
            output.status.success(),
            "{program} {args:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A git repository at `name` whose committed mise.toml requests dummy 1, with
    /// 1.0.0 installed, and which mise tracks.
    fn project(&self, name: &str) -> PathBuf {
        let dir = self.dir.join(name);
        fs::create_dir_all(&dir).expect("the project is created");
        fs::write(dir.join("mise.toml"), "[tools]\ndummy = \"1\"\n").expect("mise.toml is written");
        self.command("git", &["init", "-q"], &dir);
        self.command("git", &["add", "."], &dir);
        self.command(
            "git",
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "init",
            ],
            &dir,
        );
        self.command("mise", &["install", "dummy@1.0.0"], &dir);
        self.command("mise", &["ls"], &dir);
        dir
    }

    fn current(&self, dir: &Path) -> String {
        self.command("mise", &["current", "dummy"], dir)
            .trim()
            .to_owned()
    }

    /// Runs `mise-multi-upgrade` with `args` and `input` as stdin, after failing the test
    /// if mise tracks any config outside this home, which a run that answers yes would
    /// upgrade, should the isolation above ever stop holding.
    fn run(&self, args: &[&str], input: &str) -> Run {
        let tracked = self.command("mise", &["config", "ls", "--tracked-configs"], &self.dir);
        for path in tracked.lines() {
            assert!(
                path.starts_with(&format!("{}/", self.dir.display())),
                "{path}"
            );
        }
        let mut child = self
            .env(env!("CARGO_BIN_EXE_dotfiles-local"))
            .arg("mise-multi-upgrade")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("dotfiles-local starts");
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let _ = stdin.write_all(input.as_bytes());
        drop(stdin);
        let output = child.wait_with_output().expect("dotfiles-local ends");
        Run {
            code: output.status.code(),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

impl Drop for RealMise {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Whether `text` has a line that, split on whitespace, gives `cells`.
fn has_row(text: &str, cells: &[&str]) -> bool {
    text.lines()
        .any(|line| line.split_whitespace().collect::<Vec<_>>() == cells)
}

#[test]
fn shows_a_projects_table_from_the_real_mise_and_upgrades_it_within_its_request() {
    let real = RealMise::new("upgrade");
    let project = real.project("project");

    let run = real.run(&["--include-tmpdirs", "--dry-run"], "");
    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        has_row(&run.out, &["dummy", "1", "1.0.0", "-", "1.1.0", "-"]),
        "{}",
        run.out
    );

    let run = real.run(&["--include-tmpdirs", "--dry-run", "--bump"], "");
    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        has_row(
            &run.out,
            &["dummy", "1", "1.0.0", "-", "1.1.0", "-", "2.0.0", "-"]
        ),
        "{}",
        run.out
    );

    let run = real.run(&["--include-tmpdirs"], "y\n");
    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(
        run.out.contains(&format!(
            "==> mise -C {} upgrade --no-global",
            project.display()
        )),
        "{}",
        run.out
    );
    assert_eq!(real.current(&project), "1.1.0");
    assert_eq!(
        fs::read_to_string(project.join("mise.toml")).expect("mise.toml is read"),
        "[tools]\ndummy = \"1\"\n"
    );
}

#[test]
fn leaves_out_a_linked_worktree_that_the_real_git_reports() {
    let real = RealMise::new("worktree");
    let project = real.project("project");
    let worktree = project.join(".claude/worktrees/wt");
    let worktree_arg = worktree.display().to_string();
    real.command("git", &["worktree", "add", "-q", &worktree_arg], &project);
    real.command("mise", &["ls"], &worktree);
    let tracked = real.command("mise", &["config", "ls", "--tracked-configs"], &real.dir);
    assert!(tracked.contains(&worktree_arg), "{tracked}");

    let run = real.run(&["--include-tmpdirs", "--dry-run"], "");
    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(!run.out.contains(&worktree_arg), "{}", run.out);
    assert!(
        run.out.contains(&project.display().to_string()),
        "{}",
        run.out
    );

    let run = real.run(
        &["--include-tmpdirs", "--dry-run", "--include-worktrees"],
        "",
    );
    assert_eq!(run.code, Some(0), "{}", run.err);
    assert!(run.out.contains(&worktree_arg), "{}", run.out);
}
