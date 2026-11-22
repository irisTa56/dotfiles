//! The dotfiles checkout a command acts on, found from where it runs.

use std::fs;
use std::path::PathBuf;

use dotfiles_shared::Failure;

use crate::git::read;

/// The manifest of dotfiles-local, which marks a dotfiles checkout.
const MANIFEST: &str = "rust/crates/local/Cargo.toml";

/// The checkout holding the directory a command runs in, a worktree as much as the
/// main one, known to be a dotfiles one.
pub(crate) struct DotfilesCheckout {
    root: PathBuf,
}

impl DotfilesCheckout {
    /// The checkout holding the directory the command runs in, which has to be a
    /// dotfiles one: a command that acts on dotfiles' own files would act on another
    /// repository's in that one. A checkout counts as dotfiles where it holds
    /// dotfiles-local's own source.
    pub(crate) fn find() -> Result<Self, Failure> {
        let root = PathBuf::from(read(&["rev-parse", "--show-toplevel"])?);
        let holds_source = fs::read_to_string(root.join(MANIFEST)).is_ok_and(|manifest| {
            manifest
                .lines()
                .any(|line| line.trim() == "name = \"dotfiles-local\"")
        });
        if !holds_source {
            return Err(Failure::new(
                format_args!(
                    "[fail] {} is not a dotfiles checkout: no {MANIFEST} naming dotfiles-local",
                    root.display()
                ),
                1,
            ));
        }
        Ok(DotfilesCheckout { root })
    }

    /// The main checkout of the repository, the first worktree git lists, which
    /// outlives any other worktree of it.
    pub(crate) fn main(&self) -> Result<PathBuf, Failure> {
        let root = self.root.to_string_lossy();
        let list = read(&["-C", &root, "worktree", "list", "--porcelain"])?;
        list.lines()
            .next()
            .and_then(|line| line.strip_prefix("worktree "))
            .map(PathBuf::from)
            .ok_or_else(|| Failure::new(format_args!("[fail] git lists no main checkout"), 1))
    }

    /// The checkout's top-level directory.
    pub(crate) fn into_root(self) -> PathBuf {
        self.root
    }
}
