//! `dotfiles-tasks pin-check --bump`, run as a built binary in repositories each test
//! sets up: outside GitHub Actions it moves the pins in the working tree, and under it
//! pushes the move on origin's default branch to a branch of its own.

mod support;

use std::ffi::OsStr;

use tempfile::TempDir;

use support::checkout::{Checkout, Run};
use support::git::IDENTITY;
use support::origin::Origin;
use support::upstream::{Upstream, marker};

/// The branch a bump under GitHub Actions pushes its move to.
const BUMP: &str = "upstream-pin-bump";

/// Where the bump runs, which decides where it moves the pins.
#[derive(Clone, Copy)]
enum Runner {
    Local,
    GitHubActions,
}

impl Runner {
    /// Runs `pin-check --bump` in `checkout`, with `env` set over what the runner sets.
    fn bump(self, checkout: &Checkout, env: &[(&str, Option<&OsStr>)]) -> Run {
        let actions = match self {
            Runner::Local => None,
            Runner::GitHubActions => Some(OsStr::new("true")),
        };
        checkout.run_with(&["--bump"], &[&[("GITHUB_ACTIONS", actions)], env].concat())
    }
}

#[test]
fn moves_a_differing_pin_to_the_tip_in_every_file_that_spells_it() {
    let up = Upstream::changing_lints(2);
    let other = Upstream::new(&[&[("x/a", Some("a"))]]);
    let kept = other.marker("x", 0);
    let cargo = format!("# {}\n# {kept}\n", up.marker("lints", 0));
    let clippy = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo), ("clippy.toml", &clippy)]);
    Runner::Local.bump(&checkout, &[]).expect(
        1,
        &[&format!(
            "moved the pin of {}//lints to {}",
            up.url(),
            up.commit(1)
        )],
    );
    let tip = up.marker("lints", 1);
    assert_eq!(
        String::from_utf8_lossy(&checkout.read("Cargo.toml")),
        format!("# {tip}\n# {kept}\n")
    );
    assert_eq!(
        String::from_utf8_lossy(&checkout.read("clippy.toml")),
        format!("# {tip}\n")
    );
}

// The checkout is often a pull request's merge commit, so the move is made on the
// default branch as origin has it, and the working tree is left alone.
#[test]
fn under_github_actions_pushes_the_move_on_origins_default_branch_to_its_own_branch() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let pinned = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &pinned), ("README.md", "r")]);
    checkout.publish(&origin);
    Runner::GitHubActions.bump(&checkout, &[]).expect(
        1,
        &[
            "pushed the move to upstream-pin-bump",
            &format!("{}//lints to {}", up.url(), up.commit(1)),
            "/compare/upstream-pin-bump?expand=1",
        ],
    );
    assert_eq!(checkout.read("Cargo.toml"), pinned.as_bytes());
    assert_eq!(
        origin.show(&format!("{BUMP}:Cargo.toml")),
        Some(format!("# {}\n", up.marker("lints", 1)).into_bytes())
    );
    assert_eq!(
        origin.show(&format!("{BUMP}:README.md")),
        Some(b"r".to_vec())
    );
    assert_eq!(origin.commit(&format!("{BUMP}~1")), origin.commit("main"));
}

// The link a person follows from the bump is GitHub's comparison of the two commits,
// and the commit is the bot's, since a runner has no git identity of its own; the
// upstream is reached at a github.com URL that git's config rewrites to a local one.
#[test]
fn under_github_actions_commits_as_the_bot_and_links_githubs_comparison() {
    let up = Upstream::changing_lints(2);
    let url = "https://github.com/o/up.git";
    let aux = TempDir::new().expect("a temporary directory can be made");
    let config = aux.path().join("gitconfig");
    std::fs::write(
        &config,
        format!("[url \"{}\"]\n\tinsteadOf = {url}\n", up.url()),
    )
    .expect("the config can be written");
    let origin = Origin::new();
    let cargo = format!("# {}\n", marker(url, "lints", up.commit(0)));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout.publish(&origin);
    Runner::GitHubActions
        .bump(
            &checkout,
            &[
                ("GITHUB_SERVER_URL", Some(OsStr::new("https://github.com"))),
                ("GITHUB_REPOSITORY", Some(OsStr::new("o/consumer"))),
                ("GIT_CONFIG_GLOBAL", Some(config.as_os_str())),
                ("GIT_CONFIG_NOSYSTEM", Some(OsStr::new("1"))),
            ],
        )
        .expect(
            1,
            &[
                &format!(
                    "what it brings: https://github.com/o/up/compare/{}...{}",
                    up.commit(0), up.commit(1)
                ),
                "open its pull request: https://github.com/o/consumer/compare/upstream-pin-bump?expand=1",
            ],
        );
    let bot = "github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>";
    assert_eq!(
        origin.log(BUMP, "%an <%ae>").as_deref(),
        Some(bot),
        "author"
    );
    assert_eq!(
        origin.log(BUMP, "%cn <%ce>").as_deref(),
        Some(bot),
        "committer"
    );
}

/// What a file that is not UTF-8 text holds before its pin, and after it.
const NOT_UTF8: [&[u8]; 2] = [b"# caf\xe9\n# ", b"\n# \xff\xfe\n"];

// A file that is not UTF-8 text may still hold a pin; moving the pin must leave every
// other byte of it as it was.
#[test]
fn keeps_every_byte_of_a_file_outside_the_pin_it_moves() {
    let up = Upstream::changing_lints(2);
    let checkout = Checkout::holding(
        "Cargo.toml",
        &[NOT_UTF8[0], up.marker("lints", 0).as_bytes(), NOT_UTF8[1]].concat(),
    );
    Runner::Local.bump(&checkout, &[]).expect(1, &[]);
    assert_eq!(
        checkout.read("Cargo.toml"),
        [NOT_UTF8[0], up.marker("lints", 1).as_bytes(), NOT_UTF8[1]].concat()
    );
}

#[test]
fn under_github_actions_pushes_every_byte_of_a_file_outside_the_pin_it_moves() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::holding(
        "Cargo.toml",
        &[NOT_UTF8[0], up.marker("lints", 0).as_bytes(), NOT_UTF8[1]].concat(),
    );
    checkout.publish(&origin);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    assert_eq!(
        origin.show(&format!("{BUMP}:Cargo.toml")),
        Some([NOT_UTF8[0], up.marker("lints", 1).as_bytes(), NOT_UTF8[1]].concat())
    );
}

// Later steps of the job may read history from the checkout, so the bump must leave how
// much of it the repository holds as it found it.
#[test]
fn under_github_actions_leaves_a_full_checkouts_history_as_it_was() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout.publish(&origin);
    for message in ["second", "third"] {
        checkout.git(
            &[
                &IDENTITY[..],
                &["commit", "-q", "--allow-empty", "-m", message],
            ]
            .concat(),
        );
    }
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    assert_eq!(
        checkout.git(&["rev-parse", "--is-shallow-repository"]),
        "false"
    );
    assert_eq!(checkout.git(&["rev-list", "--count", "HEAD"]), "3");
}

// A shallow checkout, as actions/checkout leaves by default, may be behind origin's
// default branch; the bump takes that branch's tip alone rather than the history
// between, which the checkout was made without.
#[test]
fn under_github_actions_leaves_a_shallow_checkout_holding_the_default_branch_tip_alone() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::with_main(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    let checkout = Checkout::shallow_clone(&origin);
    origin.advance_main(3);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    let tip = origin.commit("main").expect("origin has main");
    assert_eq!(
        checkout.git(&["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    assert_eq!(checkout.git(&["rev-list", "--count", &tip]), "1");
}

// A pin may sit in a script, whose executable bit the pushed move must keep.
#[test]
fn under_github_actions_pushes_a_moved_file_at_the_mode_it_has_on_the_default_branch() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let script = format!("#!/bin/sh\n# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("run.sh", &script)]);
    checkout.git(&["update-index", "--chmod=+x", "run.sh"]);
    checkout.publish(&origin);
    // The checkout, such as a pull request's merge commit, may stage the file at
    // another mode; the move takes the default branch's.
    checkout.git(&["update-index", "--chmod=-x", "run.sh"]);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    assert_eq!(origin.mode(BUMP, "run.sh").as_deref(), Some("100755"));
}

/// A commit spelled as a pin can spell one, which no upstream in these tests has.
const ABSENT_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// A pin it could not tell about leaves the run unable to tell, while the bump still
// moves the pin that differs.
#[test]
fn moves_a_differing_pin_and_still_fails_as_unable_to_tell_beside_an_unreadable_one() {
    let up = Upstream::changing_lints(2);
    let aux = TempDir::new().expect("a temporary directory can be made");
    let missing = format!("file://{}", aux.path().join("missing").display());
    let unreadable = format!("# {}\n", marker(&missing, "x", ABSENT_COMMIT));
    let cargo = format!("# {}\n{unreadable}", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    Runner::Local.bump(&checkout, &[]).expect(
        2,
        &[&format!(
            "moved the pin of {}//lints to {}",
            up.url(),
            up.commit(1)
        )],
    );
    assert_eq!(
        String::from_utf8_lossy(&checkout.read("Cargo.toml")),
        format!("# {}\n{unreadable}", up.marker("lints", 1))
    );
}

// A token without contents: write, or a fork's pull request, gets the push refused.
#[test]
fn under_github_actions_fails_as_unable_to_tell_where_origin_refuses_the_push() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    origin.refuse_pushes_to(BUMP);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout.publish(&origin);
    Runner::GitHubActions
        .bump(&checkout, &[])
        .expect(2, &["could not push the move to upstream-pin-bump"]);
    assert_eq!(origin.commit(BUMP), None);
}

#[test]
fn under_github_actions_leaves_an_existing_bump_branch_alone() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout.publish(&origin);
    checkout.git(&["push", "-q", "origin", &format!("main:{BUMP}")]);
    Runner::GitHubActions
        .bump(&checkout, &[])
        .expect(1, &["upstream-pin-bump already exists on origin"]);
    assert_eq!(origin.commit(BUMP), origin.commit("main"));
}

// A pin added on a pull request's branch is not on the default branch yet.
#[test]
fn under_github_actions_fails_as_unable_to_tell_where_the_default_branch_holds_no_pin_to_move() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("README.md", "r"), ("Cargo.toml", &cargo)]);
    // The pin is staged only after origin got main without it.
    checkout.git(&["rm", "-q", "--cached", "Cargo.toml"]);
    checkout.publish(&origin);
    checkout.git(&["add", "Cargo.toml"]);
    Runner::GitHubActions
        .bump(&checkout, &[])
        .expect(2, &["holds none of these pins to move"]);
}

// The move would leave a pin at a commit that does not hold the path, which the next
// check reports as unable to tell.
#[test]
fn leaves_a_pin_whose_path_is_gone_from_the_default_branch_where_it_was() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("lints/a.toml", None)]]);
    let pinned = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &pinned)]);
    Runner::Local.bump(&checkout, &[]).expect(1, &[]);
    assert_eq!(checkout.read("Cargo.toml"), pinned.as_bytes());
}

// The checkout is often a pull request's merge commit, so its pin can be one that
// origin's default branch no longer holds; the move starts where the default branch's
// file is.
#[test]
fn under_github_actions_names_the_commit_origins_default_branch_pins_as_where_the_move_starts() {
    let up = Upstream::changing_lints(3);
    let origin = Origin::with_main(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 1)))]);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.stage_over(&origin);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    let message = origin.log(BUMP, "%B").expect("origin has the bump branch");
    assert!(message.contains(up.commit(1)), "{message}");
    assert!(!message.contains(up.commit(0)), "{message}");
}

#[test]
fn under_github_actions_claims_no_move_of_a_pin_origins_default_branch_does_not_hold() {
    let a = Upstream::changing_lints(2);
    let b = Upstream::new(&[&[("y/a", Some("a"))], &[("y/a", Some("b"))]]);
    let origin = Origin::with_main(&[("Cargo.toml", &format!("# {}\n", a.marker("lints", 0)))]);
    let cargo = format!("# {}\n# {}\n", a.marker("lints", 0), b.marker("y", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout.stage_over(&origin);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    let message = origin.log(BUMP, "%B").expect("origin has the bump branch");
    assert!(message.contains(&a.url()), "{message}");
    assert!(!message.contains(&b.url()), "{message}");
}

// A file larger than a pipe's buffer is read back from git and hashed again whole, so
// the move must not stall on its size or leave that file's pin behind.
#[test]
fn under_github_actions_moves_a_pin_in_a_large_file_beside_one_in_a_small_file() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let padding = format!("# {}\n", "x".repeat(1_600_000));
    let big = format!("{padding}# {}\n", up.marker("lints", 0));
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("big.toml", &big), ("Cargo.toml", &cargo)]);
    checkout.publish(&origin);
    Runner::GitHubActions.bump(&checkout, &[]).expect(1, &[]);
    assert!(
        origin.show(&format!("{BUMP}:big.toml"))
            == Some(format!("{padding}# {}\n", up.marker("lints", 1)).into_bytes()),
        "the pushed big.toml does not hold the moved pin"
    );
    assert_eq!(
        origin.show(&format!("{BUMP}:Cargo.toml")),
        Some(format!("# {}\n", up.marker("lints", 1)).into_bytes())
    );
}

// What matters is the pin origin holds, whose tree may equal the tip's already.
#[test]
fn under_github_actions_pushes_no_move_of_a_pin_origin_holds_at_a_commit_whose_path_equals_the_tips()
 {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
        &[("README.md", Some("r"))],
    ]);
    let origin = Origin::with_main(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 1)))]);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.stage_over(&origin);
    Runner::GitHubActions
        .bump(&checkout, &[])
        .expect(2, &["holds none of these pins to move"]);
    assert_eq!(origin.commit(BUMP), None);
}
