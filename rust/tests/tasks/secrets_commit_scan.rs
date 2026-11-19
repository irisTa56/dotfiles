//! `tasks/secrets/commit-scan.sh`, a shell task with no subcommand: whether a commit
//! stages a secret, checked by the real gitleaks.

use crate::support::checkout::Checkout;

/// A GitHub personal access token in gitleaks' `github-pat` form, put together here so
/// that no scan of this repository finds one in its source.
fn token() -> String {
    let alphabet: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    let tail: String = (0..36).map(|n| alphabet[(n * 7 + 3) % 62]).collect();
    format!("{}_{tail}", "ghp")
}

/// A file assigning the token, as a commit might stage one.
fn leaking() -> String {
    format!("token = \"{}\"\n", token())
}

// A secret the commit stages would otherwise land in history.
#[test]
fn fails_on_a_secret_the_commit_stages() {
    Checkout::new(&[("a.txt", &leaking())])
        .secrets_commit_scan_in(".")
        .expect(1, &["github-pat", "a.txt"]);
}

// A commit staging no secret goes through.
#[test]
fn passes_a_commit_that_stages_no_secret() {
    Checkout::new(&[("a.txt", "fine\n")])
        .secrets_commit_scan_in(".")
        .expect(0, &[]);
}

// Only what the commit stages blocks it.
#[test]
fn leaves_out_a_secret_the_commit_does_not_stage() {
    let checkout = Checkout::new(&[("a.txt", "fine\n")]);
    checkout.write("b.txt", leaking());
    checkout.secrets_commit_scan_in(".").expect(0, &[]);
}

// mise runs a task at the project root, which a config below the git root puts in a
// subdirectory, and a secret staged outside it is staged all the same.
#[test]
fn fails_on_a_secret_staged_outside_the_directory_it_runs_in() {
    let checkout = Checkout::new(&[("a.txt", &leaking())]);
    checkout.make_dir("sub");
    checkout
        .secrets_commit_scan_in("sub")
        .expect(1, &["github-pat", "a.txt"]);
}

// Outside a repository there is no commit to scan, and git says so.
#[test]
fn fails_with_gits_status_outside_a_repository() {
    let checkout = Checkout::new(&[("a.txt", &leaking())]);
    checkout.remove(".git");
    checkout
        .secrets_commit_scan_in(".")
        .expect(128, &["not a git repository"]);
}
