//! `dotfiles-tasks pin-check`, run as a built binary in repositories each test sets up.

mod support;

use tempfile::TempDir;

use support::checkout::Checkout;
use support::git::git;
use support::upstream::{Upstream, marker};

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
        .run_with(&[], &[("GIT_DIR", Some(hook.path().as_os_str()))])
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
        .run_with(&[], &[("TMPDIR", Some(missing.as_os_str()))])
        .expect(2, &["[fail] stopped on"]);
}

#[test]
fn fails_as_unable_to_tell_on_an_option_it_does_not_take() {
    Checkout::new(&[])
        .run(&["--frobnicate"])
        .expect(2, &["takes --bump"]);
}
