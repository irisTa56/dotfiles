//! Runs through `tasks/upstream/pin-check.sh`, the wrapper consumers run, which installs
//! the release it pins with mise and runs it.

use crate::support::checkout::Checkout;
use crate::support::mise::{Mise, POSTINSTALL};
use crate::support::upstream::{ABSENT_COMMIT, Upstream};

// Installing the release can fail, for a version not yet published or a host no binary
// is built for, and that is no answer about the pins.
#[test]
fn fails_as_unable_to_tell_where_the_release_it_pins_cannot_be_installed() {
    let cargo = format!("# git::https://example.com/u.git//p?ref={ABSENT_COMMIT}\n");
    let run = Checkout::new(&[("Cargo.toml", &cargo)])
        .wrapper("upstream/pin-check.sh", &Mise::installing_with(1))
        .run();
    assert_eq!(run.code(), 2, "{}", run.output());
    assert_eq!(run.output(), "");
}

// A consumer narrows the files read with pathspecs, which reach the binary only as
// written: a name holding a space, split in two, would name no file at all.
#[test]
fn passes_its_arguments_to_upstream_pin_check_as_given() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let behind = format!("# {}\n", up.marker("lints", 0));
    let current = format!("# {}\n", up.marker("lints", 1));
    Checkout::new(&[("Cargo.toml", &behind), ("lints b.toml", &current)])
        .wrapper("upstream/pin-check.sh", &Mise::installing_with(0))
        .arg("lints b.toml")
        .run()
        .expect(0, &["[ok]"]);
}

// The binary's exit code is the consumer's answer, so the wrapper must end with it rather
// than with its own; and a run without --bump leaves the differing pin where it was.
#[test]
fn exits_as_upstream_pin_check_does_on_a_pin_whose_path_changed() {
    let up = Upstream::new(&[
        &[("lints/a.toml", Some("a"))],
        &[("lints/a.toml", Some("b"))],
    ]);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let checkout = Checkout::new(&[("Cargo.toml", &cargo)]);
    checkout
        .wrapper("upstream/pin-check.sh", &Mise::installing_with(0))
        .run()
        .expect(1, &[&format!("{}//lints differs", up.url())]);
    assert_eq!(checkout.read("Cargo.toml"), cargo.as_bytes());
}

// mise runs a consumer's `[hooks] postinstall` on every install in its project, and one
// that registers git hooks there stops the push `--bump` makes in CI; installing the
// release is the wrapper's business, not an occasion for the consumer's setup.
#[test]
fn installs_the_release_without_running_the_consumers_postinstall() {
    let up = Upstream::new(&[&[("lints/a.toml", Some("a"))]]);
    let cargo = format!("# {}\n", up.marker("lints", 0));
    let run = Checkout::new(&[("Cargo.toml", &cargo)])
        .wrapper("upstream/pin-check.sh", &Mise::installing_with(0))
        .run();
    assert_eq!(run.code(), 0, "{}", run.output());
    assert!(!run.output().contains(POSTINSTALL), "{}", run.output());
}
