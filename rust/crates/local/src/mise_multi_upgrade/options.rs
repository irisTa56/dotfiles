//! The options a run is given.

use super::report::Report;

/// What the arguments of a run ask for, each switch false where it was not given.
#[derive(Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a switch of its own, as the arguments give it"
)]
pub(super) struct Options {
    dry_run: bool,
    bump: bool,
    include_worktrees: bool,
    include_tmpdirs: bool,
    help: bool,
}

/// One of the switches, as the arguments name it.
#[derive(Debug, Clone, Copy)]
enum Switch {
    DryRun,
    Bump,
    IncludeWorktrees,
    IncludeTmpdirs,
    Help,
}

impl Options {
    /// The options `args` give, or `None` where they cannot be read.
    ///
    /// `--name` sets a switch and `--no-name` clears it, `--name=true` and `--name=false`
    /// set it either way, `-n` and `-h` may run together with a value after `=` going to
    /// the last, a later switch replaces an earlier one, `--` ends the switches, and any
    /// argument that is not one is refused.
    pub(super) fn parse(args: &[String]) -> Option<Self> {
        let mut options = Options::default();
        for (index, arg) in args.iter().enumerate() {
            if arg == "--" {
                // Nothing may follow: the command takes no other argument.
                return (index + 1 == args.len()).then_some(options);
            }
            let switches = match arg.strip_prefix("--") {
                Some(long) => vec![Switch::long(long)?],
                None => Switch::short(arg.strip_prefix('-')?)?,
            };
            for (switch, on) in switches {
                options.set(switch, on);
            }
        }
        Some(options)
    }

    fn set(&mut self, switch: Switch, on: bool) {
        let slot = match switch {
            Switch::DryRun => &mut self.dry_run,
            Switch::Bump => &mut self.bump,
            Switch::IncludeWorktrees => &mut self.include_worktrees,
            Switch::IncludeTmpdirs => &mut self.include_tmpdirs,
            Switch::Help => &mut self.help,
        };
        *slot = on;
    }

    pub(super) fn help(&self) -> bool {
        self.help
    }

    /// The `mise outdated` runs a run makes, and so the columns its tables have.
    pub(super) fn report(&self) -> Report {
        if self.bump {
            Report::Bump
        } else {
            Report::Pinned
        }
    }

    pub(super) fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub(super) fn bump(&self) -> bool {
        self.bump
    }

    pub(super) fn include_worktrees(&self) -> bool {
        self.include_worktrees
    }

    pub(super) fn include_tmpdirs(&self) -> bool {
        self.include_tmpdirs
    }
}

impl Switch {
    /// The switch a long option names after its `--`, and whether it sets or clears it.
    fn long(option: &str) -> Option<(Self, bool)> {
        let (name, value) = match option.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (option, None),
        };
        if let Some(switch) = Self::named(name) {
            return Some((switch, Self::value(value)?));
        }
        // `--no-name` takes no value.
        let switch = Self::named(name.strip_prefix("no-")?)?;
        value.is_none().then_some((switch, false))
    }

    fn named(name: &str) -> Option<Self> {
        match name {
            "dry-run" => Some(Switch::DryRun),
            "bump" => Some(Switch::Bump),
            "include-worktrees" => Some(Switch::IncludeWorktrees),
            "include-tmpdirs" => Some(Switch::IncludeTmpdirs),
            "help" => Some(Switch::Help),
            _ => None,
        }
    }

    /// The switches a short option names after its `-`, each set but the last, which a
    /// value after `=` may set or clear.
    fn short(option: &str) -> Option<Vec<(Self, bool)>> {
        let (letters, value) = match option.split_once('=') {
            Some((letters, value)) => (letters, Some(value)),
            None => (option, None),
        };
        let mut switches = letters
            .chars()
            .map(|letter| match letter {
                'n' => Some((Switch::DryRun, true)),
                'h' => Some((Switch::Help, true)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let last = switches.last_mut()?;
        last.1 = Self::value(value)?;
        Some(switches)
    }

    /// Whether a switch given `value` after its `=`, if any, is set.
    fn value(value: Option<&str>) -> Option<bool> {
        match value {
            None | Some("true") => Some(true),
            Some("false") => Some(false),
            Some(_) => None,
        }
    }
}
