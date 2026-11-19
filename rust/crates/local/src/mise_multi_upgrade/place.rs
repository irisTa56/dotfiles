//! Where a run looks for tools to upgrade.

/// The global config, or a directory holding a tracked project config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Place {
    Global,
    Dir(String),
}

impl Place {
    /// What the place is called in a table's heading, a question and a report, a
    /// directory under the home directory named from `~`.
    pub(super) fn label(&self) -> String {
        match self {
            Place::Global => "global".to_owned(),
            Place::Dir(dir) => {
                let home = std::env::var("HOME").ok().filter(|home| !home.is_empty());
                match home.and_then(|home| dir.strip_prefix(&home)?.strip_prefix('/')) {
                    Some(rest) => format!("~/{rest}"),
                    None => dir.clone(),
                }
            }
        }
    }

    /// The arguments `mise outdated` is run with to ask, of the view `mise upgrade`
    /// acts on here, what it would upgrade. The global config is asked from `/`, where
    /// no project config is loaded; a project leaves the global config out, so a
    /// global tool is reported once.
    pub(super) fn outdated_args(&self) -> Vec<String> {
        match self {
            Place::Global => args(&["-C", "/", "outdated", "--global"]),
            Place::Dir(dir) => args(&["-C", dir, "outdated", "--no-global"]),
        }
    }

    /// The arguments `mise upgrade` is run with here.
    pub(super) fn upgrade_args(&self) -> Vec<String> {
        match self {
            Place::Global => args(&["upgrade", "--global"]),
            Place::Dir(dir) => args(&["-C", dir, "upgrade", "--no-global"]),
        }
    }
}

fn args(args: &[&str]) -> Vec<String> {
    args.iter().map(|&arg| arg.to_owned()).collect()
}
