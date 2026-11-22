//! `dotfiles-tasks links-commit-check`: whether the links of the files a commit stages
//! resolve, checked by lychee, and the wrapper `tasks/links/commit-check.sh`.

mod failures;
mod wrapper;

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::support::checkout::Checkout;
use crate::support::git::IDENTITY;
use crate::support::invocation::Stream;

/// The repository's lychee config: offline, so a test never waits on a host, and lychee
/// still resolves a link to a local file.
const OFFLINE: (&str, &str) = ("lychee.toml", "offline = true\n");

/// A repository holding `files` beside the offline lychee config, all staged.
fn staged(files: &[(&str, &str)]) -> Checkout {
    Checkout::new(&[&[OFFLINE], files].concat())
}

/// Where `program` is found on PATH.
fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set"))
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .expect("the program is on PATH")
}

// A link the commit breaks would otherwise land, and a link it does not add is the one
// case that wants this check alone skipped, so the failure says how. mise names the
// task it runs by the task's own name, whichever name or alias it was called by, and
// without the `.sh` the skip needs, so the skip follows that name rather than one the
// binary was built with.
#[test]
fn fails_on_a_broken_link_in_a_staged_file_naming_how_to_skip_the_task_mise_runs() {
    staged(&[("a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("MISE_TASK_NAME", "links:commit-check")
        .run()
        .expect(
            1,
            &[
                "missing.md",
                "MISE_TASK_SKIP=links:commit-check.sh git commit",
            ],
        );
}

// An inline task named with the extension, as one that overrides the shared task with a
// build of its own does, is skipped by that name as it stands.
#[test]
fn names_an_inline_task_named_with_the_extension_as_it_stands() {
    staged(&[("a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("MISE_TASK_NAME", "links:commit-check.sh")
        .run()
        .expect(1, &["MISE_TASK_SKIP=links:commit-check.sh git commit"]);
}

// Run other than by mise, there is no task for a skip to name, so the failure names
// none rather than one that may skip nothing.
#[test]
fn names_no_skip_when_mise_is_not_running_a_task() {
    let run = staged(&[("a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env_remove("MISE_TASK_NAME")
        .run();
    assert_eq!(run.code(), 1, "{}", run.output());
    assert!(!run.output().contains("MISE_TASK_SKIP"), "{}", run.output());
}

// An empty name is no task either, and `.sh` alone would skip nothing.
#[test]
fn names_no_skip_when_mise_names_an_empty_task() {
    let run = staged(&[("a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("MISE_TASK_NAME", "")
        .run();
    assert_eq!(run.code(), 1, "{}", run.output());
    assert!(!run.output().contains("MISE_TASK_SKIP"), "{}", run.output());
}

// The repository's lychee config decides which files a run over the whole tree reads,
// and a commit's check reads no file that run would leave out.
#[test]
fn leaves_out_a_staged_file_the_lychee_config_excludes() {
    let run = Checkout::new(&[
        (
            "lychee.toml",
            "offline = true\nexclude_path = [\"drafts/\"]\n",
        ),
        ("drafts/a.md", "[gone](missing.md)\n"),
    ])
    .dotfiles_tasks()
    .arg("links-commit-check")
    .run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// lychee reads an input as a glob, so a name holding `[`, `*` or `?` would otherwise
// name other files, or none, and its links would go unchecked.
#[test]
fn checks_a_staged_file_whose_name_holds_glob_characters() {
    staged(&[("c[1]*?.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &["c[1]*?.md", "missing.md"]);
}

// mise runs a task from the directory it is called in, and a commit stages files across
// the repository, so a run from a subdirectory checks them all, from the root.
#[test]
fn checks_the_files_staged_across_the_repository_when_run_from_a_subdirectory() {
    let checkout = staged(&[
        ("a.md", "[gone](missing.md)\n"),
        ("sub/b.md", "[up](../a.md)\n"),
    ]);
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .in_dir("sub")
        .run()
        .expect(1, &["a.md", "missing.md"]);
}

// `lychee .` skips what .gitignore ignores, so a file forced into the index past it is
// left out here too.
#[test]
fn leaves_out_a_staged_file_gitignore_ignores() {
    let checkout = staged(&[(".gitignore", "ignored.md\n")]);
    checkout.write("ignored.md", "[gone](missing.md)\n");
    checkout.git(&["add", "-f", "ignored.md"]);
    let run = checkout.dotfiles_tasks().arg("links-commit-check").run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// Only what the commit stages blocks it.
#[test]
fn leaves_out_a_broken_link_in_a_file_the_commit_does_not_stage() {
    let checkout = staged(&[]);
    checkout.write("untracked.md", "[gone](missing.md)\n");
    let run = checkout.dotfiles_tasks().arg("links-commit-check").run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// A file the commit removes from the index leaves with its links, though it stays on
// disk for lychee to list.
#[test]
fn leaves_out_a_file_the_commit_removes() {
    let checkout = staged(&[("a.md", "[gone](missing.md)\n")]);
    checkout.git(&[&IDENTITY[..], &["commit", "-qm", "base"]].concat());
    checkout.git(&["rm", "-q", "--cached", "a.md"]);
    let run = checkout.dotfiles_tasks().arg("links-commit-check").run();
    assert_eq!((run.code(), run.output()), (0, ""));
}

// lychee reads a listed line starting with `#` as a comment.
#[test]
fn checks_a_staged_file_whose_name_starts_with_a_hash() {
    staged(&[("#a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &["#a.md", "missing.md"]);
}

// git quotes a name outside ASCII unless told not to, and lychee finds no file by the
// quoted name.
#[test]
fn checks_a_staged_file_whose_name_is_outside_ascii() {
    staged(&[("é.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &["é.md", "missing.md"]);
}

// A pre-commit hook names the index the commit is made from, which `git commit <path>`
// makes apart from the usual one.
#[test]
fn checks_the_files_staged_in_the_index_a_hook_names() {
    let checkout = staged(&[("a.md", "[gone](missing.md)\n")]);
    let git_dir = checkout.git(&["rev-parse", "--absolute-git-dir"]);
    let index = format!("{git_dir}/other-index");
    fs::copy(format!("{git_dir}/index"), &index).expect("the index can be copied");
    checkout.git(&["rm", "-q", "--cached", "a.md"]);
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("GIT_INDEX_FILE", &index)
        .run()
        .expect(1, &["a.md", "missing.md"]);
}

// lychee reads a file from disk, so an edit left unstaged is checked with the rest.
#[test]
fn checks_a_staged_file_as_it_is_on_disk() {
    let checkout = staged(&[("a.md", "fine\n")]);
    checkout.write("a.md", "[gone](missing.md)\n");
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &["a.md", "missing.md"]);
}

// Outside a repository there is no commit to check, and checking the directory instead
// would be no answer about one.
#[test]
fn fails_with_gits_status_outside_a_repository() {
    let checkout = staged(&[("a.md", "fine\n")]);
    checkout.remove(".git");
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(128, &["not a git repository"]);
}

// lychee comes from the task's tool header, so a consumer whose mise did not put it on
// PATH has to be told that no link was checked, as a shell says of a missing command.
#[test]
fn fails_as_a_shell_does_on_a_missing_command_where_lychee_is_not_on_path() {
    let checkout = staged(&[("a.md", "[gone](missing.md)\n")]);
    let bin = TempDir::new().expect("a temporary directory can be made");
    std::os::unix::fs::symlink(which("git"), bin.path().join("git")).expect("git can be linked");
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("PATH", bin.path())
        .run()
        .expect(127, &["lychee"]);
}

// A lychee config lychee cannot load leaves the files unlisted, and lychee's own status
// says so, as it did when the task was a shell pipeline.
#[test]
fn fails_with_lychees_status_where_lychee_cannot_load_the_config() {
    Checkout::new(&[("lychee.toml", "offline = \"yes\"\n"), ("a.md", "fine\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(3, &["lychee.toml"]);
}

// A repository's files can name more than one command line holds, Linux's 2 MiB and
// macOS's 1 MiB, and the files past it are checked all the same.
#[test]
fn checks_more_staged_files_than_one_command_line_can_name() {
    // Each path is some 800 bytes, short of macOS's 1024-byte limit on one.
    let dir = ["d".repeat(200), "e".repeat(200), "f".repeat(200)].join("/");
    let mut files: Vec<(String, &str)> = (0..4000)
        .map(|n| (format!("{dir}/{n:0>200}.md"), "fine\n"))
        .collect();
    files.push((format!("{dir}/last.md"), "[gone](missing.md)\n"));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, body)| (path.as_str(), *body))
        .collect();
    staged(&files)
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &["last.md", "missing.md"]);
}

// A caller may close stderr, and a broken link still has to fail the commit.
#[test]
fn fails_on_a_broken_link_when_stderr_is_closed() {
    let run = staged(&[("a.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .closing(Stream::Stderr)
        .run();
    assert_eq!(run.code(), 1, "output: {}", run.output());
}

// A lychee on PATH that cannot be run checks no link either, and a shell ends such a
// command with its own status.
#[test]
fn fails_as_a_shell_does_on_a_command_it_cannot_run_where_lychee_is_not_executable() {
    let checkout = staged(&[("a.md", "[gone](missing.md)\n")]);
    let bin = TempDir::new().expect("a temporary directory can be made");
    std::os::unix::fs::symlink(which("git"), bin.path().join("git")).expect("git can be linked");
    fs::write(bin.path().join("lychee"), "").expect("the stand-in can be written");
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .env("PATH", bin.path())
        .run()
        .expect(126, &["lychee"]);
}

// git reads a pathspec starting with `:` as magic, so a name like that would match
// nothing, and its links would go unchecked.
#[test]
fn checks_a_staged_file_whose_name_git_could_read_as_pathspec_magic() {
    staged(&[(":x.md", "[gone](missing.md)\n")])
        .dotfiles_tasks()
        .arg("links-commit-check")
        .run()
        .expect(1, &[":x.md", "missing.md"]);
}

// Each commit would otherwise request every link of its files again, and the
// repository ignores the cache where it sits, at the root. lychee keeps no entry for a
// link to a local file, but writes the file all the same.
#[test]
fn keeps_lychees_cache_at_the_root_when_run_from_a_subdirectory() {
    let checkout = staged(&[
        ("a.md", "[b](b.md)\n"),
        ("b.md", "fine\n"),
        ("sub/c.md", "fine\n"),
    ]);
    checkout
        .dotfiles_tasks()
        .arg("links-commit-check")
        .in_dir("sub")
        .run()
        .expect(0, &[]);
    assert_eq!(
        (
            checkout.exists(".lycheecache"),
            checkout.exists("sub/.lycheecache")
        ),
        (true, false)
    );
}
