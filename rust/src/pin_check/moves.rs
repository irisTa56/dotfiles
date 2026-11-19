//! Saying what a bump moved.

use std::sync::LazyLock;

use regex::Regex;

use super::{Moved, Moves};

static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(https://github\.com/[^/]+/[^/]+?)(?:\.git)?$")
        .expect("GITHUB is a valid pattern")
});

impl Moved {
    /// What the move brings, as a page where the host has one.
    fn brings(&self) -> String {
        let Moved { url, from, tip, .. } = self;
        match GITHUB.captures(url) {
            Some(github) => format!("{}/compare/{from}...{tip}", &github[1]),
            None => format!("{url} {from}...{tip}"),
        }
    }
}

impl Moves {
    /// One line per distinct move, however many files or markers it touched.
    pub(super) fn describe(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for moved in &self.0 {
            let line = format!(
                "{}//{} to {}; what it brings: {}",
                moved.url,
                moved.path,
                moved.tip,
                moved.brings()
            );
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
        lines
    }
}
