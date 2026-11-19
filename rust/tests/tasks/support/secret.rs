//! A secret for the scanners to find, put together at run time so that no scan of this
//! repository finds one in its source.

/// A GitHub personal access token in the form gitleaks' `github-pat` rule and
/// trufflehog's `Github` detector match.
pub(crate) fn github_token() -> String {
    let alphabet: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    let tail: String = (0..36).map(|n| alphabet[(n * 7 + 3) % 62]).collect();
    format!("{}_{tail}", "ghp")
}
