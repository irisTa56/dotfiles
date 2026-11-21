//! The upgrades a run was asked for.

use std::io::{self, Write};

use super::mise::Mise;
use super::place::Place;
use crate::failure::Failure;

/// The places answered yes, in order.
pub(super) struct Upgrades {
    places: Vec<Place>,
}

impl Upgrades {
    pub(super) fn new(places: Vec<Place>) -> Self {
        Upgrades { places }
    }

    /// Runs `mise upgrade` in each place, keeping each version request as it is, and
    /// returns those where it failed: one failure leaves the other places worth
    /// upgrading.
    pub(super) fn run(self) -> Result<Vec<Place>, Failure> {
        let mut stdout = io::stdout();
        if self.places.is_empty() {
            writeln!(stdout, "Nothing upgraded.").map_err(|error| Failure::writing(&error))?;
            return Ok(Vec::new());
        }
        let mut failed = Vec::new();
        for place in self.places {
            let mise = Mise::new(place.upgrade_args());
            writeln!(stdout, "\n==> {mise}").map_err(|error| Failure::writing(&error))?;
            let status = mise.pass_on()?;
            if !status.success() {
                failed.push(place);
            }
        }
        Ok(failed)
    }
}
