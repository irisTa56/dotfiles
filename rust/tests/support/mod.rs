//! What the integration tests share: the repositories they set up, and runs of the
//! built binary in them.

// Each file under tests/ is a crate of its own, which compiles all of this.
#![allow(dead_code, reason = "each test crate uses its own share of these")]

mod checkout;
mod git;
mod upstream;

pub(crate) use checkout::Checkout;
pub(crate) use git::git;
pub(crate) use upstream::{Upstream, marker};
