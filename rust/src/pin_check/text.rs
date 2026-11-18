//! Finding the markers in a text, and moving them.

use std::fmt;
use std::ops::Range;
use std::sync::LazyLock;

use regex::bytes::Regex;

use super::{Marker, Moved, Moves, Text};

/// A marker is the form mise's `git::` include takes, so a `task_config.includes` entry is
/// one as it stands. The URL is a scheme, a host and the path segments after it, ending at
/// the first `//` past the scheme's, which is where mise splits it too; requiring the scheme
/// keeps a URL from opening with `-`, which git would take as an option. Only a full sha
/// counts: a branch or a short sha moves, so there would be no fixed commit to compare
/// against, and [`Text::markers`] drops a match that more hex digits follow. In Unicode
/// mode a class matches only valid UTF-8, so each group it captures is UTF-8.
static MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"git::([a-z][a-z0-9+.-]*://[^\s"'?/]*(?:/[^\s"'?/]+)*?)//([^\s"'?]+)\?ref=([0-9a-f]{40})"#,
    )
    .expect("MARKER is a valid pattern")
});

impl fmt::Display for Marker<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "git::{}//{}?ref={}", self.url, self.path, self.commit)
    }
}

impl<'t> Text<'t> {
    /// The markers in the text, each with the range it spans.
    pub(super) fn markers(self) -> impl Iterator<Item = (Range<usize>, Marker<'t>)> {
        let text = self.0;
        MARKER.captures_iter(text).filter_map(move |found| {
            let whole = found.get(0).expect("a match has a whole");
            let more_hex = text[whole.end()..]
                .first()
                .is_some_and(|next| next.is_ascii_digit() || (b'a'..=b'f').contains(next));
            let group = |n| {
                let bytes = found.get(n).expect("a marker has three groups").as_bytes();
                std::str::from_utf8(bytes).expect("MARKER captures only UTF-8")
            };
            (!more_hex).then(|| {
                (
                    whole.range(),
                    Marker {
                        url: group(1),
                        path: group(2),
                        commit: group(3),
                    },
                )
            })
        })
    }

    /// Moves each marker that `tip_for` gives a tip for, and returns the moved text with
    /// what moved, each move read from the marker it replaced.
    pub(super) fn rewrite(
        self,
        mut tip_for: impl FnMut(&Marker) -> Option<String>,
    ) -> (Vec<u8>, Moves) {
        let text = self.0;
        let mut moved = Moves::default();
        let mut result = Vec::with_capacity(text.len());
        let mut last = 0;
        for (range, found) in self.markers() {
            let Some(tip) = tip_for(&found).filter(|tip| tip != found.commit) else {
                continue;
            };
            result.extend_from_slice(&text[last..range.start]);
            result.extend_from_slice(
                Marker {
                    commit: &tip,
                    ..found
                }
                .to_string()
                .as_bytes(),
            );
            last = range.end;
            moved.0.push(Moved {
                url: found.url.to_owned(),
                path: found.path.to_owned(),
                from: found.commit.to_owned(),
                tip,
            });
        }
        result.extend_from_slice(&text[last..]);
        (result, moved)
    }
}
