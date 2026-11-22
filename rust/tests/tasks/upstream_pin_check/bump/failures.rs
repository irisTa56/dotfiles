//! How each end of a run with `--bump` is reported: the exit code, and stdout and stderr,
//! each whole. git and curl are stand-ins where the end is one the real ones cannot be
//! made to reach on cue.

use super::{BUMP, OPENED, Runner, without_curl};
use crate::support::checkout::Checkout;
use crate::support::github::GitHubApi;
use crate::support::origin::Origin;
use crate::support::stand_in::{FAILING, KILLED, StandIns};
use crate::support::upstream::Upstream;

/// An upstream whose `lints` changed after its first commit, an origin whose `main` pins
/// it at that commit in `Cargo.toml`, and a clone of origin.
fn pinned_on_origin() -> (Upstream, Origin, Checkout) {
    let up = Upstream::changing_lints(2);
    let origin = Origin::with_main(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    let checkout = Checkout::clone_of(&origin);
    (up, origin, checkout)
}

/// What the check says of `up`'s `lints` pinned at its first commit in `Cargo.toml`.
fn differs(up: &Upstream) -> String {
    format!(
        "[fail] {}//lints differs from the default branch, pinned in Cargo.toml\n[fail] pinned: {}\n[fail] tip:    {}\n",
        up.url(),
        up.commit(0),
        up.commit(1)
    )
}

/// The line that says what moving that pin to the tip brings.
fn move_line(up: &Upstream) -> String {
    format!(
        "{u}//lints to {c1}; what it brings: {u} {c0}...{c1}",
        u = up.url(),
        c0 = up.commit(0),
        c1 = up.commit(1)
    )
}

#[test]
fn names_the_move_and_exits_1_where_it_moves_a_pin_in_the_working_tree() {
    let up = Upstream::changing_lints(2);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    Runner::Local.bump(&checkout).run_apart().expect(
        1,
        "",
        &format!(
            "{}[fail] moved the pin of {}\n",
            differs(&up),
            move_line(&up)
        ),
    );
}

#[test]
fn says_what_stopped_it_and_exits_2_where_it_cannot_write_the_moved_file() {
    let up = Upstream::changing_lints(2);
    let checkout = Checkout::new(&[("Cargo.toml", &format!("# {}\n", up.marker("lints", 0)))]);
    checkout.protect("Cargo.toml");
    Runner::Local.bump(&checkout).run_apart().expect(
        2,
        "",
        &format!(
            "{}[fail] stopped on Permission denied (os error 13)\n",
            differs(&up)
        ),
    );
}

#[test]
fn under_github_actions_names_the_push_and_the_pull_request_and_exits_1() {
    let (up, _origin, checkout) = pinned_on_origin();
    Runner::GitHubActions.bump(&checkout).run_apart().expect(
        1,
        "",
        &format!(
            "{}[fail] pushed the move to {BUMP}: {}\n[fail] opened its pull request: {OPENED}\n",
            differs(&up),
            move_line(&up)
        ),
    );
}

#[test]
fn under_github_actions_names_the_push_onto_an_existing_bump_branch_and_exits_1() {
    let (up, origin, checkout) = pinned_on_origin();
    origin.commit_onto_main_as(BUMP, "README.md", "r");
    Runner::GitHubActions.bump(&checkout).run_apart().expect(
        1,
        "",
        &format!(
            "{}[fail] pushed the move onto {BUMP}: {}\n",
            differs(&up),
            move_line(&up)
        ),
    );
}

#[test]
fn under_github_actions_says_a_bump_branch_holding_the_tips_is_left_and_exits_1() {
    let (up, origin, checkout) = pinned_on_origin();
    origin.commit_onto_main_as(
        BUMP,
        "Cargo.toml",
        &format!("# {}\n", up.marker("lints", 1)),
    );
    Runner::GitHubActions.bump(&checkout).run_apart().expect(
        1,
        "",
        &format!(
            "{}[fail] {BUMP} holds none of these pins to move, or holds them where the path matches the tip's already, so it is left as it is\n",
            differs(&up)
        ),
    );
}

/// Asserts that a bump under GitHub Actions in `checkout`, finding the programs it runs
/// on the tests' PATH behind `bin`, exits 2 having said the check's lines of `up` and
/// then `end`.
fn bump_ends(up: &Upstream, checkout: &Checkout, bin: &StandIns, end: &str) {
    Runner::GitHubActions
        .bump(checkout)
        .env("PATH", bin.path())
        .run_apart()
        .expect(2, "", &format!("{}{end}", differs(up)));
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_tell_if_the_branch_exists()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("--exit-code", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!("stand-in: failed\n[fail] could not tell whether {BUMP} exists on origin\n"),
    );
}

#[test]
fn under_github_actions_exits_2_where_a_signal_stops_git_asking_if_the_branch_exists() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("--exit-code", KILLED);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!("[fail] could not tell whether {BUMP} exists on origin\n"),
    );
}

#[test]
fn under_github_actions_says_the_token_is_needed_and_exits_2_without_it() {
    let (up, _origin, checkout) = pinned_on_origin();
    Runner::GitHubActions
        .bump(&checkout)
        .env_remove("GITHUB_TOKEN")
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}[fail] opening the pull request needs the job's token in the step's env, as GITHUB_TOKEN: ${{{{ github.token }}}}\n",
                differs(&up)
            ),
        );
}

#[test]
fn under_github_actions_says_the_repository_is_needed_and_exits_2_without_it() {
    let (up, _origin, checkout) = pinned_on_origin();
    Runner::GitHubActions
        .bump(&checkout)
        .env_remove("GITHUB_REPOSITORY")
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}[fail] opening the pull request needs GITHUB_REPOSITORY, which GitHub Actions sets\n",
                differs(&up)
            ),
        );
}

#[test]
fn under_github_actions_says_curl_is_needed_and_exits_2_without_it() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = without_curl();
    Runner::GitHubActions
        .bump(&checkout)
        .env("PATH", bin.path())
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}[fail] opening the pull request needs curl on PATH\n",
                differs(&up)
            ),
        );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_tell_origins_default_branch()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("--symref", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not tell which branch is origin's default\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_tell_if_the_checkout_is_shallow()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("--is-shallow-repository", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not tell whether the checkout is shallow\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_fetch_the_default_branch()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("fetch -q origin", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not fetch HEAD from origin\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_fetch_the_bump_branch()
{
    let (up, origin, checkout) = pinned_on_origin();
    origin.commit_onto_main_as(BUMP, "README.md", "r");
    let bin = StandIns::new().git_on("fetch -q origin", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!("stand-in: failed\n[fail] could not fetch refs/heads/{BUMP} from origin\n"),
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_read_the_default_branch()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("read-tree", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not read origin's default branch\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_list_a_file_there() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("ls-tree", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not read Cargo.toml on origin's default branch\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_read_a_file_there() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("cat-file blob", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not read Cargo.toml on origin's default branch\n",
    );
}

// On a bump branch that exists already, the move is read from that branch, which the
// line names.
#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_read_the_bump_branch() {
    let (up, origin, checkout) = pinned_on_origin();
    origin.commit_onto_main_as(BUMP, "README.md", "r");
    let bin = StandIns::new().git_on("read-tree", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!("stand-in: failed\n[fail] could not read {BUMP}\n"),
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_list_a_file_on_the_bump_branch()
 {
    let (up, origin, checkout) = pinned_on_origin();
    origin.commit_onto_main_as(BUMP, "README.md", "r");
    let bin = StandIns::new().git_on("ls-tree", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!("stand-in: failed\n[fail] could not read Cargo.toml on {BUMP}\n"),
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_stage_the_move() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("update-index", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not stage the move of Cargo.toml\n",
    );
}

#[test]
fn under_github_actions_says_the_default_branch_holds_no_pin_to_move_and_exits_2() {
    let up = Upstream::changing_lints(2);
    let origin = Origin::new();
    let checkout = Checkout::new(&[
        ("README.md", "r"),
        ("Cargo.toml", &format!("# {}\n", up.marker("lints", 0))),
    ]);
    checkout.git(&["rm", "-q", "--cached", "Cargo.toml"]);
    checkout.publish(&origin);
    checkout.git(&["add", "Cargo.toml"]);
    bump_ends(
        &up,
        &checkout,
        &StandIns::new(),
        "[fail] origin's default branch holds none of these pins to move, or holds them where the path matches the tip's already\n",
    );
}

// The stand-in reads the message it is fed before it fails, as git does, since a git
// that exits first fails the run's write of it and the run says that instead.
#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_commit_the_move() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("commit-tree", &format!("/bin/cat >/dev/null\n{FAILING}"));
    bump_ends(
        &up,
        &checkout,
        &bin,
        "stand-in: failed\n[fail] could not commit the move\n",
    );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_push_the_move() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().git_on("push -q origin", FAILING);
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!(
            "stand-in: failed\n[fail] could not push the move to {BUMP}; the push needs the job's token with contents: write kept on the checkout's origin\n"
        ),
    );
}

/// What a run says once it has pushed the move and GitHub refused its pull request with
/// a 403 saying `refusal`.
fn refused(up: &Upstream, refusal: &str) -> String {
    format!(
        "[fail] pushed the move to {BUMP}: {}\n[fail] could not open its pull request: GitHub refused it with HTTP 403: {refusal}; opening one needs the job's token with pull-requests: write, and the repository setting \"Allow GitHub Actions to create and approve pull requests\"\n",
        move_line(up)
    )
}

#[test]
fn under_github_actions_says_github_refused_the_pull_request_and_the_branch_is_deleted_and_exits_2()
{
    let (up, _origin, checkout) = pinned_on_origin();
    let api = GitHubApi::answering(403, r#"{"message":"Resource not accessible"}"#);
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}{}[fail] deleted {BUMP} again, so that the next run opens its pull request\n",
                differs(&up),
                refused(&up, "Resource not accessible")
            ),
        );
}

#[test]
fn under_github_actions_passes_on_what_git_said_and_exits_2_where_it_cannot_delete_the_branch_again()
 {
    let (up, _origin, checkout) = pinned_on_origin();
    let api = GitHubApi::answering(403, r#"{"message":"Resource not accessible"}"#);
    let bin = StandIns::new().git_on("push -q origin :refs/heads/", FAILING);
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .env("PATH", bin.path())
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}{}stand-in: failed\n[fail] could not delete {BUMP}; delete it by hand, or the next run will open no pull request\n",
                differs(&up),
                refused(&up, "Resource not accessible")
            ),
        );
}

#[test]
fn under_github_actions_passes_on_what_curl_said_and_exits_2_where_curl_fails() {
    let (up, _origin, checkout) = pinned_on_origin();
    let bin = StandIns::new().program(
        "curl",
        "[ \"$1\" = --version ] && exit 0\necho 'curl: (7) stand-in' >&2\nexit 7\n",
    );
    bump_ends(
        &up,
        &checkout,
        &bin,
        &format!(
            "[fail] pushed the move to {BUMP}: {}\n[fail] could not open its pull request: curl: (7) stand-in\n[fail] deleted {BUMP} again, so that the next run opens its pull request\n",
            move_line(&up)
        ),
    );
}

// The stand-in curl removes itself once it has said it runs, so the run that opens the
// pull request finds no curl.
#[test]
fn under_github_actions_says_curl_could_not_run_and_exits_2_where_curl_is_gone_by_the_request() {
    let (up, _origin, checkout) = pinned_on_origin();
    let rest = without_curl();
    let bin = StandIns::new().program("curl", "/bin/rm \"$0\"\n");
    Runner::GitHubActions
        .bump(&checkout)
        .env("PATH", bin.path_over(rest.path().as_os_str()))
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}[fail] pushed the move to {BUMP}: {}\n[fail] could not open its pull request: could not run curl: No such file or directory (os error 2)\n[fail] deleted {BUMP} again, so that the next run opens its pull request\n",
                differs(&up),
                move_line(&up)
            ),
        );
}

#[test]
fn under_github_actions_says_githubs_answer_named_no_pull_request_and_exits_2() {
    let (up, _origin, checkout) = pinned_on_origin();
    let api = GitHubApi::answering(201, "{}");
    Runner::GitHubActions
        .bump(&checkout)
        .env("GITHUB_API_URL", api.url())
        .run_apart()
        .expect(
            2,
            "",
            &format!(
                "{}[fail] pushed the move to {BUMP}: {}\n[fail] could not open its pull request: GitHub's answer named no pull request\n[fail] deleted {BUMP} again, so that the next run opens its pull request\n",
                differs(&up),
                move_line(&up)
            ),
        );
}
