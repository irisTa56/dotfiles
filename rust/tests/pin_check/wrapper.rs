//! Runs through `tasks/upstream/pin-check.sh`, the wrapper consumers run, which installs
//! the release it pins with mise and runs it.

use crate::support::checkout::Checkout;
use crate::support::mise::Mise;
use crate::support::upstream::{ABSENT_COMMIT, Upstream};

// Installing the release can fail, for a version not yet published or a host no binary
// is built for, and that is no answer about the pins.
#[test]
fn fails_as_unable_to_tell_where_the_release_it_pins_cannot_be_installed() {
    let cargo = format!("# git::https://example.com/u.git//p?ref={ABSENT_COMMIT}\n");
    let run = Checkout::new(&[("Cargo.toml", &cargo)]).run_wrapper(&[], &Mise::installing_with(1));
    assert_eq!(run.code(), 2, "{}", run.output());
    assert_eq!(run.output(), "");
}

// A consumer narrows the files read with pathspecs, which reach the binary only as
// written: a name holding a space, split in two, would name no file at all.
#[test]
fn passes_its_arguments_to_pin_check_as_given() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let behind = format!("# {}\n", up.marker("lints", 0));
    let current = format!("# {}\n", up.marker("lints", 1));
    Checkout::new(&[("Cargo.toml", &behind), ("lints b.toml", &current)])
        .run_wrapper(&["lints b.toml"], &Mise::installing_with(0))
        .expect(0, &["[ok]"]);
}

// The binary's exit code is the consumer's answer, so the wrapper must end with it rather
// than with its own; and a run without --bump leaves the differing pin where it was.
#[test]
fn exits_as_pin_check_does_on_a_pin_whose_path_changed() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout
        .run_wrapper(&[], &Mise::installing_with(0))
        .expect(1, &[&format!("{}//lints differs", up.url())]);
    assert_eq!(checkout.read("Cargo.toml"), cargo.as_bytes());
}
