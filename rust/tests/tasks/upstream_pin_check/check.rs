//! Runs without `--bump`: reading the pins and comparing each with its upstream.

use tempfile::TempDir;

use crate::support::checkout::Checkout;
use crate::support::git::git;
use crate::support::invocation::Stream;
use crate::support::upstream::{ABSENT_COMMIT, Upstream, marker};

// A wrong pathspec, or a pin spelled so that it is not read, would otherwise pass having
// checked nothing.
#[test]
fn fails_when_no_tracked_file_holds_a_pin() {
    Checkout::new(&[(
        "Cargo.toml",
        "# upstream: git::https://example.com/a.git//lints?ref=main\n",
    )])
    .dotfiles_tasks()
    .arg("upstream-pin-check")
    .run()
    .expect(2, &["found no upstream pin"]);
}

// git grep exits 1 where no tracked file spells git:: at all, which is no failure of
// git's, so the run says the same as for a marker it does not read.
#[test]
fn fails_when_no_tracked_file_spells_a_marker_at_all() {
    let run = Checkout::new(&[("Cargo.toml", "[package]\n")])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run();
    assert_eq!(run.code(), 2, "{}", run.output());
    assert_eq!(
        run.output(),
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .env("GIT_DIR", hook.path().as_os_str())
        .run()
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
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
        .expect(
            1,
            &[
                &format!("{}//lints differs", up.url()),
                &format!("pinned: {}", up.commit(0)),
                &format!("tip:    {}", up.commit(1)),
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
        .expect(1, &["is gone from the default branch"]);
}

#[test]
fn fails_as_unable_to_tell_where_the_upstream_cannot_be_read() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let url = format!("file://{}", aux.path().join("missing").display());
    let cargo = format!("# {}\n", marker(&url, "lints", ABSENT_COMMIT));
    // git's own reason comes first, so the run says why it could not tell.
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
        .expect(
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
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
        .expect(
            2,
            &[&format!("could not read {ABSENT_COMMIT} from file://")],
        );
}

#[test]
fn fails_as_unable_to_tell_where_the_path_is_not_in_the_pinned_commit() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let cargo = format!("# {}\n", up.marker("lint", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .arg("mise.toml")
        .run()
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
    .dotfiles_tasks()
    .arg("upstream-pin-check")
    .run()
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
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
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .run()
        .expect(0, &["[ok]"]);
}

#[test]
fn fails_as_unable_to_tell_where_it_cannot_make_its_scratch_repository() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let aux = TempDir::new().expect("a temporary directory can be made");
    let missing = aux.path().join("no-such-dir");
    let cargo = format!("# {}\n", up.marker("lints", 0));
    Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .env("TMPDIR", missing.as_os_str())
        .run()
        .expect(2, &["[fail] stopped on"]);
}

#[test]
fn fails_as_a_usage_error_on_an_option_it_does_not_take() {
    Checkout::new(&[])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .arg("--frobnicate")
        .run()
        .expect(2, &["'--frobnicate'"]);
}

// An option takes effect only before the pathspecs, so a `--bump` written last moves
// nothing, and the run says so rather than checking without it.
#[test]
fn fails_as_a_usage_error_on_bump_after_a_pathspec() {
    let up = Upstream::changing_lints(2);
    let pinned = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &pinned)]);
    checkout
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .args(["Cargo.toml", "--bump"])
        .run()
        .expect(
            2,
            &[
                "'--bump'",
                "--bump, --draft and --branch-suffix, then git pathspecs",
            ],
        );
    assert_eq!(checkout.read("Cargo.toml"), pinned.as_bytes());
}

#[test]
fn fails_as_a_usage_error_on_a_dash_argument_after_a_pathspec() {
    Checkout::new(&[])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .args(["Cargo.toml", "-x"])
        .run()
        .expect(2, &["'-x'"]);
}

// A reader that stops early, as `| head` does, closes stdout before the pins are all
// checked, and the pins after an [ok] line it could not take are still checked.
#[test]
fn checks_the_pins_after_an_ok_line_when_stdout_is_closed() {
    let tasks = Upstream::new(&[&[("tasks/a.sh", Some("a"))]]);
    let lints = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    // Pins are checked in the order the file spells them, so the [ok] line comes first.
    let cargo = format!(
        "# {}\n# {}\n",
        tasks.marker("tasks", 0),
        lints.marker("lints", 0)
    );
    let run = Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .closing(Stream::Stdout)
        .run();
    run.expect(1, &[&format!("{}//lints differs", lints.url())]);
}

// A caller that reads only stdout may close stderr, and the run still has to end with
// the status that says a pin differs.
#[test]
fn fails_on_a_pin_whose_path_changed_when_stderr_is_closed() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let cargo = format!("# upstream: {}\n", up.marker("lints", 0));
    let run = Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .closing(Stream::Stderr)
        .run();
    assert_eq!((run.code(), run.output()), (1, ""));
}

// git's own reason goes to stderr ahead of the run's line, so it too is dropped rather
// than ending the run on a stderr its reader closed.
#[test]
fn fails_as_unable_to_tell_where_the_upstream_cannot_be_read_when_stderr_is_closed() {
    let aux = TempDir::new().expect("a temporary directory can be made");
    let url = format!("file://{}", aux.path().join("missing").display());
    let cargo = format!("# {}\n", marker(&url, "lints", ABSENT_COMMIT));
    let run = Checkout::new(&[("Cargo.toml", &cargo)])
        .dotfiles_tasks()
        .arg("upstream-pin-check")
        .closing(Stream::Stderr)
        .run();
    assert_eq!((run.code(), run.output()), (2, ""));
}

// --draft and --branch-suffix say how --bump opens its pull request, so alone either would do
// nothing.
#[test]
fn fails_as_a_usage_error_on_draft_or_branch_suffix_without_bump() {
    let up = Upstream::changing_lints(2);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    for args in [&["--draft"][..], &["--branch-suffix", "b"][..]] {
        checkout
            .dotfiles_tasks()
            .arg("upstream-pin-check")
            .args(args)
            .run()
            .expect(2, &["required arguments were not provided", "--bump"]);
    }
}
