//! Which `mise outdated` runs a run makes, and so which columns its tables have.

/// The versions a run reports beside each tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Report {
    /// Those within each tool's request, which `mise upgrade` installs.
    Pinned,
    /// Those within the request, and the newest overall, reachable only by changing it.
    Bump,
}
