//! `with-new-terminal`: a shell script run in a new Terminal window, through an
//! osascript that stands in for macOS's, since the tests run on Linux too.

use std::fs;
use std::path::Path;
use std::process::Output;

use crate::support::{Sandbox, dotfiles_local};

/// A directory of the tests' own, holding the script to run and a fake osascript.
struct Fixture {
    sandbox: Sandbox,
    /// Whether the tests' own PATH follows the sandbox's `bin`, which leaves out
    /// macOS's osascript where the sandbox has none.
    path: SearchPath,
}

#[derive(Clone, Copy)]
enum SearchPath {
    Inherited,
    SandboxOnly,
}

/// What a run left: its output, and the script osascript was handed to run, if it ran.
struct Run {
    output: Output,
    applescript: Option<String>,
}

impl Fixture {
    /// A fresh sandbox named after `test`, whose osascript ends with `exit` (a shell
    /// command, such as `exit 3`).
    fn new(test: &str, exit: &str) -> Self {
        let sandbox = Sandbox::new("with-new-terminal", test, &[]);
        fs::write(
            sandbox.root().join("job.sh"),
            "#!/bin/zsh\n# a comment\n\n  # an indented comment\necho \"a\\b\" && ls\n",
        )
        .expect("the script is written");
        // It records its arguments one per NUL, and prints what macOS's prints for a
        // new window, which no caller of with-new-terminal reads.
        sandbox.fake(
            "osascript",
            &format!(
                "printf '%s\\0' \"$@\" > \"$(dirname \"$0\")/args\"\necho 'tab 1 of window id 1'\n{exit}"
            ),
        );
        Fixture {
            sandbox,
            path: SearchPath::Inherited,
        }
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// This sandbox with no osascript to be found.
    fn without_osascript(self) -> Self {
        fs::remove_file(self.root().join("bin/osascript")).expect("the fake osascript is removed");
        Fixture {
            path: SearchPath::SandboxOnly,
            ..self
        }
    }

    fn script(&self) -> String {
        self.root().join("job.sh").display().to_string()
    }

    /// Runs `with-new-terminal` from the sandbox, finding the fake osascript first on PATH.
    fn run(&self, args: &[&str]) -> Run {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs = vec![self.sandbox.bin()];
        if let SearchPath::Inherited = self.path {
            dirs.extend(std::env::split_paths(&path));
        }
        let output = dotfiles_local("with-new-terminal")
            .args(args)
            .current_dir(self.root())
            .env("HOME", self.root())
            .env(
                "PATH",
                std::env::join_paths(dirs).expect("no directory on PATH holds a colon"),
            )
            .output()
            .expect("dotfiles-local starts");
        let applescript = fs::read(self.root().join("bin/args")).ok().map(|args| {
            let args = String::from_utf8(args).expect("the arguments are UTF-8");
            let args: Vec<&str> = args.trim_end_matches('\0').split('\0').collect();
            match args.as_slice() {
                ["-e", applescript] => (*applescript).to_owned(),
                other => panic!("osascript was handed {other:?}"),
            }
        });
        Run {
            output,
            applescript,
        }
    }
}

impl Run {
    fn code(&self) -> Option<i32> {
        self.output.status.code()
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }
}

#[test]
fn runs_the_script_in_a_new_window_without_its_comment_and_blank_lines() {
    let sandbox = Fixture::new("runs", "exit 0");
    let run = sandbox.run(&[&sandbox.script()]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    assert_eq!(
        run.applescript.as_deref(),
        Some(concat!(
            "tell application \"Terminal\"\n",
            "  do script \"setopt no_bang_hist && clear\\necho \\\"a\\\\b\\\" && ls\"\n",
            "  set custom title of front window to \"job.sh\"\n",
            "end tell\n",
        ))
    );
    // What osascript prints of the new window is not passed on.
    assert_eq!(run.output.stdout, b"");
}

#[test]
fn changes_to_cwd_first_and_takes_the_title_from_title() {
    let sandbox = Fixture::new("cwd-title", "exit 0");
    let run = sandbox.run(&[&sandbox.script(), "--cwd", "/srv/app", "--title", "a \"b\""]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    let applescript = run.applescript.expect("osascript ran");
    assert!(
        applescript.contains("do script \"setopt no_bang_hist && clear\\ncd /srv/app\\necho"),
        "{applescript}"
    );
    assert!(
        applescript.contains("set custom title of front window to \"a \\\"b\\\"\""),
        "{applescript}"
    );
}

// The shell function leaves the directory it was called from as the command's own.
#[test]
fn reads_a_relative_script_path_from_the_directory_it_runs_in() {
    let sandbox = Fixture::new("relative", "exit 0");
    let run = sandbox.run(&["job.sh"]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    assert!(run.applescript.expect("osascript ran").contains("&& ls"));
}

#[test]
fn fails_with_osascripts_status_when_it_fails() {
    let sandbox = Fixture::new("osascript-fails", "exit 3");
    let run = sandbox.run(&[&sandbox.script()]);
    assert_eq!(
        (run.code(), run.stderr().as_str()),
        (Some(3), "osascript failed (exit 3)\n")
    );
}

#[test]
fn refuses_anything_but_one_script_path_with_its_usage_and_exit_1() {
    let sandbox = Fixture::new("usage", "exit 0");
    for args in [
        &[][..],
        &["a.sh", "b.sh"],
        &["a.sh", "--bogus"],
        &["a.sh", "--cwd"],
        &["a.sh", "--title", "--cwd", "x"],
        &["job.sh", "--from", "."],
    ] {
        let run = sandbox.run(args);
        assert_eq!(run.code(), Some(1), "args {args:?}");
        assert!(
            run.stderr().starts_with(
                "Usage: with-new-terminal <script.sh> [--cwd <dir>] [--title <title>]\n"
            ),
            "args {args:?}: {}",
            run.stderr()
        );
        assert_eq!(run.applescript, None, "args {args:?}");
    }
}

// Options are read anywhere, with `=` or not, the last one standing, and none after
// `--`.
#[test]
fn reads_options_anywhere_and_with_an_equals_sign() {
    let sandbox = Fixture::new("options", "exit 0");
    let run = sandbox.run(&[
        "--title=first",
        "--cwd",
        "/srv/app",
        "--title",
        "-1",
        "--",
        "job.sh",
    ]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    let applescript = run.applescript.expect("osascript ran");
    assert!(
        applescript.contains("clear\\ncd /srv/app\\n"),
        "{applescript}"
    );
    assert!(applescript.contains("window to \"-1\""), "{applescript}");
}

#[test]
fn fails_with_exit_1_on_a_script_it_cannot_read() {
    let sandbox = Fixture::new("missing", "exit 0");
    let run = sandbox.run(&["missing.sh"]);
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().starts_with("cannot read missing.sh: "),
        "{}",
        run.stderr()
    );
    assert_eq!(run.applescript, None);
}

// A value may open with `-` where a digit, a space or nothing follows it.
#[test]
fn takes_a_value_that_opens_with_a_dash_where_the_mix_task_took_it() {
    let sandbox = Fixture::new("dash-values", "exit 0");
    for value in ["-", "- x", "-1abc"] {
        let run = sandbox.run(&["job.sh", "--title", value]);
        assert_eq!(run.code(), Some(0), "value {value:?}: {}", run.stderr());
        let applescript = run.applescript.expect("osascript ran");
        assert!(
            applescript.contains(&format!("window to \"{value}\"")),
            "value {value:?}: {applescript}"
        );
    }
}

// A byte that is not UTF-8 in a comment line goes with the line.
#[test]
fn runs_a_script_that_is_not_utf8() {
    let sandbox = Fixture::new("not-utf8", "exit 0");
    fs::write(sandbox.root().join("latin.sh"), b"echo hi\n# caf\xe9\n")
        .expect("the script is written");
    let run = sandbox.run(&["latin.sh"]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    assert!(
        run.applescript
            .expect("osascript ran")
            .contains("do script \"setopt no_bang_hist && clear\\necho hi\""),
    );
}

// A `~` the shell left alone, as in a quoted path, stands for the home directory.
#[test]
fn reads_a_script_path_under_home_that_opens_with_a_tilde() {
    let sandbox = Fixture::new("tilde", "exit 0");
    let run = sandbox.run(&["~/job.sh"]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    assert!(
        run.applescript
            .expect("osascript ran")
            .contains("window to \"job.sh\"")
    );
}

// A signal ends it with the status a shell gives a command the signal ended.
#[test]
fn fails_with_128_and_the_signal_when_a_signal_ends_osascript() {
    let sandbox = Fixture::new("osascript-killed", "kill -9 $$");
    let run = sandbox.run(&["job.sh"]);
    assert_eq!(
        (run.code(), run.stderr().as_str()),
        (Some(137), "osascript failed (exit 137)\n")
    );
}

#[test]
fn fails_with_exit_1_where_there_is_no_osascript() {
    let sandbox = Fixture::new("no-osascript", "exit 0").without_osascript();
    let run = sandbox.run(&["job.sh"]);
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().starts_with("osascript failed: "),
        "{}",
        run.stderr()
    );
}

// What follows the last command, such as a CR line end or spaces, is left out.
#[test]
fn leaves_out_whitespace_after_the_last_command() {
    let sandbox = Fixture::new("trailing", "exit 0");
    fs::write(sandbox.root().join("crlf.sh"), "echo a\r\nls  \r\n").expect("the script is written");
    let run = sandbox.run(&["crlf.sh"]);
    assert_eq!(run.code(), Some(0), "stderr: {}", run.stderr());
    assert!(
        run.applescript
            .expect("osascript ran")
            .contains("do script \"setopt no_bang_hist && clear\\necho a\r\\nls\"\n"),
    );
}
