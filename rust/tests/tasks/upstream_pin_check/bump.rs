//! Runs with `--bump`: outside GitHub Actions it moves the pins in the working tree, and
//! under it pushes the move on origin's default branch to a branch of its own.

use tempfile::TempDir;

use crate::support::checkout::Checkout;
use crate::support::git::IDENTITY;
use crate::support::github::GitHubApi;
use crate::support::invocation::Invocation;
use crate::support::origin::Origin;
use crate::support::upstream::{ABSENT_COMMIT, Upstream, marker};

/// The branch a bump under GitHub Actions pushes its move to.
const BUMP: &str = "upstream-pin-bump";

/// Where the bump runs, which decides where it moves the pins.
#[derive(Clone, Copy)]
enum Runner {
    Local,
    GitHubActions,
}

/// The repository a run under GitHub Actions is for, as `GITHUB_REPOSITORY` names it.
const REPOSITORY: &str = "o/consumer";
/// The pull request the stand-in API says it opened.
const OPENED: &str = "https://github.com/o/consumer/pull/7";

impl Runner {
    /// A run of `upstream-pin-check --bump` in `checkout`, with what the runner sets in
    /// its environment. Under GitHub Actions that is a token and a stand-in API that opens
    /// any pull request it is asked for.
    fn bump(self, checkout: &Checkout) -> Invocation {
        let bump = checkout
            .dotfiles_tasks()
            .args(["upstream-pin-check", "--bump"]);
        match self {
            Runner::Local => bump.env_remove("GITHUB_ACTIONS"),
            Runner::GitHubActions => {
                let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
                bump.env("GITHUB_ACTIONS", "true")
                    .env("GITHUB_TOKEN", "t0ken")
                    .env("GITHUB_API_URL", api.url())
                    .env("GITHUB_REPOSITORY", REPOSITORY)
            }
        }
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
    Runner::Local.bump(&checkout).run().expect(
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
    Runner::GitHubActions.bump(&checkout).run().expect(
        1,
        &[
            "pushed the move to upstream-pin-bump",
            &format!("{}//lints to {}", up.url(), up.commit(1)),
            &format!("opened its pull request: {OPENED}"),
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

// The pull request goes from the branch to the default branch origin's HEAD names, on
// the job's token, and its title and body are the move's commit message.
#[test]
fn under_github_actions_opens_the_pull_request_of_the_branch_it_pushes() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(1, &[&format!("opened its pull request: {OPENED}")]);
    let requests = api.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/repos/{REPOSITORY}/pulls"));
    assert_eq!(request.authorization.as_deref(), Some("Bearer t0ken"));
    assert_eq!(request.body["head"], BUMP);
    assert_eq!(request.body["base"], "main");
    assert_eq!(
        request.body["title"],
        "chore: move upstream pins to their default branches"
    );
    let body = request.body["body"].as_str().unwrap_or_default();
    assert!(
        body.contains(&format!("{}//lints to {}", up.url(), up.commit(1))),
        "{body}"
    );
}

// The pull request goes into whichever branch origin's HEAD names.
#[test]
fn under_github_actions_opens_the_pull_request_into_origins_default_branch_by_its_name() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    origin.rename_main("trunk");
    let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(1, &[]);
    let requests = api.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.body["base"], "trunk");
}

// The link the move's message gives is GitHub's comparison of the two commits, and the
// commit is the bot's, since a runner has no git identity of its own; the
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
        .bump(&checkout)
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .run()
        .expect(
            1,
            &[&format!(
                "what it brings: https://github.com/o/up/compare/{}...{}",
                up.commit(0),
                up.commit(1)
            )],
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
    Runner::Local.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
    assert_eq!(origin.mode(BUMP, "run.sh").as_deref(), Some("100755"));
}

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
    Runner::Local.bump(&checkout).run().expect(
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
        .bump(&checkout)
        .run()
        .expect(2, &["could not push the move to upstream-pin-bump"]);
    assert_eq!(origin.commit(BUMP), None);
}

// The step has to hand the job's token over; without it nothing is pushed, so the next
// run, with the token, still opens the pull request.
#[test]
fn under_github_actions_pushes_nothing_without_the_token_to_open_the_pull_request() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    Runner::GitHubActions
        .bump(&checkout)
        .env_remove("GITHUB_TOKEN")
        .run()
        .expect(2, &["GITHUB_TOKEN: ${{ github.token }}"]);
    assert_eq!(origin.commit(BUMP), None);
}

// curl is what opens the pull request, and a runner image need not have it.
#[test]
fn under_github_actions_pushes_nothing_without_curl_to_open_the_pull_request() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let bin = without_curl();
    Runner::GitHubActions
        .bump(&checkout)
        .env("PATH", bin.path())
        .run()
        .expect(2, &["needs curl on PATH"]);
    assert_eq!(origin.commit(BUMP), None);
}

// Without the repository setting GitHub refuses the job's token a pull request; the run
// says what grants it.
#[test]
fn under_github_actions_fails_as_unable_to_tell_where_github_refuses_the_pull_request() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let refusal = "GitHub Actions is not permitted to create or approve pull requests.";
    let api = GitHubApi::answering(403, &format!(r#"{{"message":"{refusal}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(
            2,
            &[
                refusal,
                "pull-requests: write",
                "Allow GitHub Actions to create and approve pull requests",
            ],
        );
}

// A branch left behind would have the next run take its pull request to be open and open
// none, so a deletion origin refuses is said, with what a person has to do.
#[test]
fn under_github_actions_says_so_where_origin_refuses_to_delete_the_branch_it_could_not_propose() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    origin.refuse_deleting(BUMP);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let api = GitHubApi::answering(403, r#"{"message":"Resource not accessible"}"#);
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(
            2,
            &["could not delete upstream-pin-bump; delete it by hand"],
        );
    assert!(origin.commit(BUMP).is_some());
}

// A refused run opens no pull request, so a later run that GitHub accepts must still open
// it, or the move waits on the branch with nobody told.
#[test]
fn under_github_actions_opens_the_pull_request_in_a_later_run_after_github_refused_one() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let refusing = GitHubApi::answering(403, r#"{"message":"Resource not accessible"}"#);
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", refusing.url())
        .run()
        .expect(2, &[]);
    let accepting = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", accepting.url())
        .run()
        .expect(1, &[OPENED]);
    let requests = accepting.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/repos/{REPOSITORY}/pulls"));
}

// A 422 says what is wrong with the request in `errors`, such as a pull request already
// open for the branch; naming a permission or setting that is not the cause sends the
// reader to change the wrong thing.
#[test]
fn under_github_actions_reports_the_reason_github_gives_for_a_422_and_blames_no_setting() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let reason = "A pull request already exists for o:upstream-pin-bump.";
    let api = GitHubApi::answering(
        422,
        &format!(
            r#"{{"message":"Validation Failed","errors":[{{"resource":"PullRequest","code":"custom","message":"{reason}"}}]}}"#
        ),
    );
    let run = Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run();
    run.expect(2, &[reason]);
    assert!(
        !run.output()
            .contains("Allow GitHub Actions to create and approve pull requests"),
        "{}",
        run.output()
    );
}

// While the branch's pull request is open, a newer tip moves the pins on that branch,
// over the commits a person added there, rather than opening another pull request.
#[test]
fn under_github_actions_moves_the_pins_on_an_existing_bump_branch_over_what_it_holds() {
    let up = Upstream::changing_lints(3);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let added = origin.commit_onto_main_as(BUMP, "NOTE.md", "kept");
    let api = GitHubApi::answering(500, "{}");
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(
            1,
            &[&format!(
                "pushed the move onto upstream-pin-bump: {}//lints to {}",
                up.url(),
                up.commit(2)
            )],
        );
    assert_eq!(origin.commit(&format!("{BUMP}~1")), Some(added));
    assert_eq!(
        origin.show(&format!("{BUMP}:Cargo.toml")),
        Some(format!("# {}\n", up.marker("lints", 2)).into_bytes())
    );
    assert_eq!(
        origin.show(&format!("{BUMP}:NOTE.md")),
        Some(b"kept".to_vec())
    );
    assert!(api.requests().is_empty(), "{:?}", api.requests());
}

// A run after the branch has the move, before its pull request merges, has nothing to add.
#[test]
fn under_github_actions_leaves_a_bump_branch_holding_the_tips_already_as_it_is() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let moved = format!("# {}\n", up.marker("lints", 1));
    let added = origin.commit_onto_main_as(BUMP, "Cargo.toml", &moved);
    Runner::GitHubActions
        .bump(&checkout)
        .run()
        .expect(1, &["upstream-pin-bump holds none of these pins to move"]);
    assert_eq!(origin.commit(BUMP), Some(added));
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
        .bump(&checkout)
        .run()
        .expect(2, &["holds none of these pins to move"]);
}

// The move would leave a pin at a commit that does not hold the path, which the next
// check reports as unable to tell.
#[test]
fn leaves_a_pin_whose_path_is_gone_from_the_default_branch_where_it_was() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))], &[("lints/a.toml", None)]]);
    let pinned = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &pinned)]);
    Runner::Local.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
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
    Runner::GitHubActions.bump(&checkout).run().expect(1, &[]);
    let pushed = origin
        .show(&format!("{BUMP}:big.toml"))
        .expect("origin's bump branch holds big.toml");
    // The file is too large to print whole, so a mismatch shows its size and its end.
    assert!(
        pushed == format!("{padding}# {}\n", up.marker("lints", 1)).into_bytes(),
        "the pushed big.toml, {} bytes, ends {:?}",
        pushed.len(),
        String::from_utf8_lossy(&pushed[pushed.len().saturating_sub(120)..])
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
        .bump(&checkout)
        .run()
        .expect(2, &["holds none of these pins to move"]);
    assert_eq!(origin.commit(BUMP), None);
}

/// A directory to take for PATH, linking every command on the tests' own PATH but curl.
/// git runs commands of its own from PATH, such as `git-upload-pack`, so they come along.
fn without_curl() -> TempDir {
    let bin = TempDir::new().expect("a temporary directory can be made");
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let link = bin.path().join(&name);
            if name != "curl" && !link.exists() {
                // A name met twice keeps the first, as PATH does; a broken entry is skipped.
                let _ = std::os::unix::fs::symlink(entry.path(), link);
            }
        }
    }
    bin
}

// A move that a person has to finish, such as one whose upstream was taken in part by
// hand, is opened as a draft, which GitHub will not merge until it is marked ready.
#[test]
fn under_github_actions_opens_the_pull_request_as_a_draft_with_draft() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .arg("--draft")
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(1, &[&format!("opened its pull request: {OPENED}")]);
    let requests = api.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.body["draft"], true);
}

// Without --draft the pull request is one a merge finishes.
#[test]
fn under_github_actions_opens_the_pull_request_ready_without_draft() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(1, &[]);
    let requests = api.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.body["draft"], false);
}

// Pins a repository wants proposed apart, such as an include a merge finishes beside a
// copy a person has to bring over, go to branches of their own, each named by a suffix to
// the task's own, so a move of one series neither lands on the other's pull request nor
// waits behind it.
#[test]
fn under_github_actions_proposes_the_move_on_the_branch_branch_suffix_names() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let other = origin.commit_onto_main_as(BUMP, "other.txt", "another series");
    let api = GitHubApi::answering(201, &format!(r#"{{"html_url":"{OPENED}"}}"#));
    Runner::GitHubActions
        .bump(&checkout)
        .args(["--branch-suffix", "lints"])
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(
            1,
            &[
                "pushed the move to upstream-pin-bump-lints",
                &format!("opened its pull request: {OPENED}"),
            ],
        );
    assert_eq!(
        origin.show("upstream-pin-bump-lints:Cargo.toml"),
        Some(format!("# {}\n", up.marker("lints", 1)).into_bytes())
    );
    assert_eq!(origin.commit(BUMP).as_deref(), Some(other.as_str()));
    let requests = api.requests();
    let [request] = &requests[..] else {
        panic!("one request, not {requests:?}");
    };
    assert_eq!(request.body["head"], "upstream-pin-bump-lints");
}

// A suffix that makes no name git takes for a branch would fail at the push, where the
// message blames the token, so it is refused by name before anything is pushed.
#[test]
fn under_github_actions_refuses_a_branch_suffix_git_cannot_take_naming_the_value() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let run = Runner::GitHubActions
        .bump(&checkout)
        .args(["--branch-suffix", "a b"])
        .run();
    assert_eq!(run.code(), 2, "exit code; output:\n{}", run.output());
    assert!(
        run.output().contains("a b"),
        "the output does not name the value:\n{}",
        run.output()
    );
    assert!(
        !run.output().contains("contents: write"),
        "the output blames the token:\n{}",
        run.output()
    );
    assert_eq!(origin.commit(BUMP), None);
    assert_eq!(origin.commit("upstream-pin-bump-a b"), None);
}

// While the pull request of the branch --branch-suffix names is open, a newer tip moves
// the pins on that branch, over what a person added there, and leaves the default bump
// branch, another series's, alone.
#[test]
fn under_github_actions_moves_the_pins_on_the_existing_branch_branch_suffix_names() {
    let up = Upstream::changing_lints(3);
    let origin = Origin::new();
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.publish(&origin);
    let added = origin.commit_onto_main_as("upstream-pin-bump-lints", "NOTE.md", "kept");
    let other = origin.commit_onto_main_as(BUMP, "other.txt", "another series");
    let api = GitHubApi::answering(500, "{}");
    Runner::GitHubActions
        .bump(&checkout)
        .args(["--branch-suffix", "lints"])
        .env("GITHUB_API_URL", api.url())
        .run()
        .expect(1, &["pushed the move onto upstream-pin-bump-lints"]);
    assert_eq!(origin.commit("upstream-pin-bump-lints~1"), Some(added));
    assert_eq!(
        origin.show("upstream-pin-bump-lints:Cargo.toml"),
        Some(format!("# {}\n", up.marker("lints", 2)).into_bytes())
    );
    assert_eq!(origin.commit(BUMP).as_deref(), Some(other.as_str()));
    assert!(api.requests().is_empty(), "{:?}", api.requests());
}
