//! `dotfiles-tasks pin-check`, run as a built binary in repositories each test sets up.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::TempDir;

/// The repository a run checks: a fresh git repository with the files a test stages.
struct Checkout {
    dir: TempDir,
}

/// How a run of the binary ended: its exit code, and stdout and stderr as one stream.
struct Run {
    code: i32,
    output: String,
}

impl Checkout {
    /// A repository holding `files`, given as path and content, staged.
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let checkout = Checkout { dir };
        checkout.git(&["init", "-q"]);
        for (path, content) in files {
            let file = checkout.dir.path().join(path);
            fs::create_dir_all(file.parent().expect("a file has a parent"))
                .expect("the file's directory can be made");
            fs::write(&file, content).expect("the file can be written");
        }
        checkout.git(&["add", "-A"]);
        checkout
    }

    /// Runs git in the repository, as setup that must succeed.
    fn git(&self, args: &[&str]) {
        git(self.dir.path(), args);
    }

    /// Runs `pin-check` with `args` in the repository.
    fn run(&self, args: &[&str]) -> Run {
        self.run_with(args, &[])
    }

    /// Runs `pin-check` with `args` in the repository, with `env` set over the
    /// isolated environment.
    fn run_with(&self, args: &[&str], env: &[(&str, &OsStr)]) -> Run {
        let (mut reader, writer) = io::pipe().expect("a pipe can be made");
        let mut command = Command::new(env!("CARGO_BIN_EXE_dotfiles-tasks"));
        command
            .arg("pin-check")
            .args(args)
            .current_dir(self.dir.path())
            .stdin(Stdio::null())
            .stdout(writer.try_clone().expect("the pipe can be shared"))
            .stderr(writer);
        isolate(&mut command, self.dir.path());
        command.envs(env.iter().copied());
        let mut child = command.spawn().expect("the binary runs");
        // The command holds the pipe's writing ends until it is dropped, and reading
        // waits for every one to close.
        drop(command);
        let mut output = String::new();
        reader
            .read_to_string(&mut output)
            .expect("the output is UTF-8");
        let status = child.wait().expect("the binary's run ends");
        Run {
            code: status
                .code()
                .expect("the binary exits rather than being killed"),
            output,
        }
    }
}

/// Keeps git, run by a test or by the binary, to the repository it is run in: a git hook
/// running the tests exports `GIT_DIR` and its kin for its own repository, and a
/// directory above the temporary one could be a repository.
fn isolate(command: &mut Command, dir: &Path) {
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.env(
        "GIT_CEILING_DIRECTORIES",
        dir.parent().expect("a temporary directory has a parent"),
    );
}

/// Runs git in `dir`, as setup that must succeed, and returns its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    isolate(&mut command, dir);
    let output = command.output().expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {dir:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git's output is UTF-8")
        .trim()
        .to_owned()
}

/// The git identity setup commits are made with, since the host may have none.
const IDENTITY: [&str; 4] = ["-c", "user.name=t", "-c", "user.email=t@example.com"];

/// An upstream repository on `main`, reached by a `file://` URL.
struct Upstream {
    dir: TempDir,
    commits: Vec<String>,
}

/// One upstream commit: each path with its content, `None` deleting the path.
type Change<'c> = &'c [(&'c str, Option<&'c str>)];

impl Upstream {
    /// A repository with one commit per item of `changes`, in order.
    fn new(changes: &[Change<'_>]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        git(dir.path(), &["init", "-q", "-b", "main"]);
        let commits = changes
            .iter()
            .map(|change| {
                for (path, content) in *change {
                    let file = dir.path().join(path);
                    match content {
                        Some(content) => {
                            fs::create_dir_all(file.parent().expect("a file has a parent"))
                                .expect("the file's directory can be made");
                            fs::write(&file, content).expect("the file can be written");
                        }
                        None => fs::remove_file(&file).expect("the file can be removed"),
                    }
                }
                git(dir.path(), &["add", "-A"]);
                git(
                    dir.path(),
                    &[&IDENTITY[..], &["commit", "-q", "--allow-empty", "-m", "c"]].concat(),
                );
                git(dir.path(), &["rev-parse", "HEAD"])
            })
            .collect();
        Upstream { dir, commits }
    }

    fn url(&self) -> String {
        format!("file://{}", self.dir.path().display())
    }

    /// The marker that pins `path` at the `index`th commit.
    fn marker(&self, path: &str, index: usize) -> String {
        marker(&self.url(), path, &self.commits[index])
    }
}

/// A marker as the tracked files spell one.
fn marker(url: &str, path: &str, commit: &str) -> String {
    format!("git::{url}//{path}?ref={commit}")
}

impl Run {
    /// Asserts that the run exited with `code` and printed each of `parts`.
    fn expect(&self, code: i32, parts: &[&str]) {
        assert_eq!(self.code, code, "exit code; output:\n{}", self.output);
        for part in parts {
            assert!(
                self.output.contains(part),
                "the output lacks {part:?}:\n{}",
                self.output
            );
        }
    }
}

// A wrong pathspec, or a pin spelled so that it is not read, would otherwise pass having
// checked nothing.
#[test]
fn fails_when_no_tracked_file_holds_a_pin() {
    Checkout::new(&[(
        "Cargo.toml",
        "# upstream: git::https://example.com/a.git//lints?ref=main\n",
    )])
    .run(&[])
    .expect(2, &["found no upstream pin"]);
}

// git grep exits 1 where no tracked file spells git:: at all, which is no failure of
// git's, so the run says the same as for a marker it does not read.
#[test]
fn fails_when_no_tracked_file_spells_a_marker_at_all() {
    let run = Checkout::new(&[("Cargo.toml", "[package]\n")]).run(&[]);
    assert_eq!(run.code, 2, "{}", run.output);
    assert_eq!(
        run.output,
        "[fail] found no upstream pin in the tracked files\n"
    );
}

// Most commits to an upstream leave the pinned path alone, and a check failing on each
// of them would get ignored.
#[test]
fn passes_a_pin_whose_path_is_unchanged_on_the_default_branch_tip() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("README.md", Some("r"))]]);
    let cargo = format!("# upstream: {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(0, &["[ok]"]);
}

// A git hook, such as a pre-push one that runs this check, exports GIT_DIR for the
// repository it runs for, which would point the scratch repository's runs at it.
#[test]
fn leaves_the_repository_of_a_git_hook_it_runs_under_alone() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("README.md", Some("r"))]]);
    let hook = TempDir::new().expect("a temporary directory can be made");
    git(hook.path(), &["init", "-q", "--bare"]);
    let cargo = format!("# upstream: {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run_with(&[], &[("GIT_DIR", hook.path().as_os_str())])
        .expect(0, &["[ok]"]);
    assert!(
        !hook.path().join("FETCH_HEAD").exists(),
        "the hook's repository was fetched into"
    );
}

// The pin may be ahead of the default branch as well as behind it, so the report names
// both commits without saying which is older.
#[test]
fn fails_on_a_pin_whose_path_changed_naming_both_commits_and_the_file() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let cargo = format!("# upstream: {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)]).run(&[]).expect(
        1,
        &[
            &format!("{}//lints differs", up.url()),
            &format!("pinned: {}", up.commits[0]),
            &format!("tip:    {}", up.commits[1]),
            "Cargo.toml",
        ],
    );
}

// Removing the path upstream is a change to it like any other.
#[test]
fn fails_on_a_pin_whose_path_is_gone_from_the_default_branch() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("lints/a.toml", None)]]);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(1, &["is gone from the default branch"]);
}

/// A commit spelled as a pin can spell one, which no upstream in these tests has.
const ABSENT_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn fails_as_unable_to_tell_where_the_upstream_cannot_be_read() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let url = format!("file://{}", aux.path().join("missing").display());
    let cargo = format!("# {}\n", marker(&url, "lints", ABSENT_COMMIT));
    // git's own reason comes first, so the run says why it could not tell.
    Checkout::new(&[("Cargo.toml", &cargo)]).run(&[]).expect(
        2,
        &[
            "does not appear to be a git repository",
            "could not read the default branch of file://",
        ],
    );
}

// A pin taken from a branch since deleted, or squash-merged, names a commit the upstream
// no longer has.
#[test]
fn fails_as_unable_to_tell_where_the_pinned_commit_is_not_in_the_upstream() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let cargo = format!("# {}\n", marker(&up.url(), "lints", ABSENT_COMMIT));
    Checkout::new(&[("Cargo.toml", &cargo)]).run(&[]).expect(
        2,
        &[&format!("could not read {ABSENT_COMMIT} from file://")],
    );
}

#[test]
fn fails_as_unable_to_tell_where_the_path_is_not_in_the_pinned_commit() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let cargo = format!("# {}\n", up.marker("lint", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(2, &["//lint does not exist at the pinned"]);
}

#[test]
fn reads_only_the_files_a_pathspec_argument_names() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let mise = format!("includes = [\"{}\"]\n", up.marker("lints", 1));
    Checkout::new(&[("Cargo.toml", &cargo), ("mise.toml", &mise)])
        .run(&["mise.toml"])
        .expect(0, &["[ok]"]);
}

// The same pin spelled in two files is one upstream question, answered once.
#[test]
fn reports_each_pin_a_pin_spelled_in_two_files_once_and_fails_if_any_differs() {
    let tasks = Upstream::new(&[&[("tasks/a.sh", Some("a"))]]);
    let lints = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let mise = format!("includes = [\"{}\"]\n", tasks.marker("tasks", 0));
    let lints_pin = format!("# {}\n", lints.marker("lints", 0));
    Checkout::new(&[
        ("mise.toml", &mise),
        ("Cargo.toml", &lints_pin),
        ("clippy.toml", &lints_pin),
    ])
    .run(&[])
    .expect(
        1,
        &[
            &format!("[ok] {}//tasks", tasks.url()),
            &format!(
                "{}//lints differs from the default branch, pinned in Cargo.toml, clippy.toml",
                lints.url()
            ),
        ],
    );
}

// Only a marker with a path is a pin; a git URL without one is some other string.
#[test]
fn reads_no_pin_from_a_git_url_that_has_no_path() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("README.md", Some("r"))]]);
    let cargo = format!(
        "# git::https://example.com/a/b.git?ref={ABSENT_COMMIT}\n# {}\n",
        up.marker("lints", 0)
    );
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(0, &["[ok]"]);
}

// A ref longer than a sha-1, such as a sha-256, is not one this compares; reading its
// first 40 digits as one would pin a commit nobody named.
#[test]
fn reads_no_pin_from_a_ref_with_more_hex_digits_than_a_sha1() {
    let cargo = format!(
        "# git::https://example.com/a.git//lints?ref={}\n",
        "0123456789abcdef".repeat(4)
    );
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(2, &["found no upstream pin"]);
}

// A URL that opens with a dash would be read by git as an option of its own.
#[test]
fn reads_no_pin_from_a_url_git_would_take_for_an_option() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("README.md", Some("r"))]]);
    let cargo = format!(
        "# git::--upload-pack=touch-me//p?ref={ABSENT_COMMIT}\n# {}\n",
        up.marker("lints", 0)
    );
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run(&[])
        .expect(0, &["[ok]"]);
}

#[test]
fn fails_as_unable_to_tell_where_it_cannot_make_its_scratch_repository() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let aux = TempDir::new().expect("a temporary directory can be made");
    let missing = aux.path().join("no-such-dir");
    let cargo = format!("# {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .run_with(&[], &[("TMPDIR", missing.as_os_str())])
        .expect(2, &[]);
}

#[test]
fn fails_as_unable_to_tell_on_an_option_it_does_not_take() {
    Checkout::new(&[])
        .run(&["--frobnicate"])
        .expect(2, &["takes --bump"]);
}
