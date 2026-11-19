//! What the integration tests share: the repositories they set up, and runs of the
//! built binary in them.

// Each file under tests/ is a crate of its own, which compiles all of this.
#![allow(dead_code, reason = "each test crate uses its own share of these")]

pub(crate) mod checkout;
pub(crate) mod git;
pub(crate) mod origin;
pub(crate) mod upstream;
