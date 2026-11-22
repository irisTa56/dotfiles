//! The directories holding a config mise tracks.

use std::path::Path;

use dotfiles_shared::Failure;

use super::mise::Mise;
use super::options::Options;
use super::place::Place;
use super::temp_dirs::TempDirs;
use crate::git;

/// The project directories of the configs mise tracks, each once, in order.
#[derive(Debug)]
pub(super) struct Tracked {
    dirs: Vec<String>,
}

impl Tracked {
    /// The project directories of the configs mise tracks, less those `options` leave
    /// out.
    pub(super) fn list(options: &Options) -> Result<Self, Failure> {
        let mut dirs: Vec<String> = Self::configs()?
            .iter()
            .map(|path| project_dir(path))
            .collect();
        dirs.sort();
        dirs.dedup();
        let temp = (!options.include_tmpdirs()).then(TempDirs::find);
        let mut kept = Vec::new();
        for dir in dirs {
            if keep(&dir, options, temp.as_ref())? {
                kept.push(dir);
            }
        }
        Ok(Tracked { dirs: kept })
    }

    /// The paths of the configs mise tracks.
    fn configs() -> Result<Vec<String>, Failure> {
        let mise = Mise::new(
            ["config", "ls", "--tracked-configs"]
                .map(str::to_owned)
                .to_vec(),
        );
        match mise.output() {
            Ok(Some(out)) => Ok(String::from_utf8_lossy(&out)
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()),
            Ok(None) => Err(Failure::new(format_args!("{mise} failed"), 1)),
            Err(failure) => Err(failure),
        }
    }

    /// The places a run asks: the global config, then each directory.
    pub(super) fn places(self) -> Vec<Place> {
        std::iter::once(Place::Global)
            .chain(self.dirs.into_iter().map(Place::Dir))
            .collect()
    }
}

/// Whether a run keeps `dir`: a temporary directory in `temp`, the ones left out, is
/// not kept, and a linked worktree only where `options` take them in.
fn keep(dir: &str, options: &Options, temp: Option<&TempDirs>) -> Result<bool, Failure> {
    if temp.is_some_and(|temp| temp.contain(dir)) {
        return Ok(false);
    }
    Ok(options.include_worktrees() || !linked_worktree(dir)?)
}

/// The directories a config may sit in under the project it configures, each with the
/// prefix of the file's name there; mise loads each of these from the project root.
const CONFIG_DIRS: [(&str, &str); 8] = [
    (".config/mise/conf.d", ""),
    (".mise/conf.d", ""),
    ("mise/conf.d", ""),
    (".config/mise", "config."),
    (".config/mise", "mise."),
    (".config", "mise."),
    (".mise", "config."),
    ("mise", "config."),
];

/// The project directory a config at `path` configures: the root of a config directory
/// it sits in, and otherwise the directory it sits in. This is where `mise upgrade` is
/// run and what is shown.
fn project_dir(path: &str) -> String {
    let path = Path::new(path);
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let Some(dir) = path.parent() else {
        return String::new();
    };
    let root = CONFIG_DIRS
        .iter()
        .find(|(suffix, prefix)| dir.ends_with(suffix) && name.starts_with(prefix))
        .and_then(|(suffix, _)| dir.ancestors().nth(Path::new(suffix).components().count()))
        .unwrap_or(dir);
    root.to_string_lossy().into_owned()
}

/// Whether `dir` is a linked worktree, an agent's short-lived copy of a checkout and not
/// a place to keep tools current: git tells one apart by a git dir of its own under the
/// common one, wherever the worktree was placed. A directory git does not know is not.
fn linked_worktree(dir: &str) -> Result<bool, Failure> {
    let args = [
        "-C",
        dir,
        "rev-parse",
        "--path-format=absolute",
        "--git-dir",
        "--git-common-dir",
    ];
    let output = git::command(&args).output()?;
    if !output.status.success() {
        return Ok(false);
    }
    let out = String::from_utf8_lossy(&output.stdout);
    match out
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()[..]
    {
        [git_dir, common_dir] => Ok(git_dir != common_dir),
        _ => Err(Failure::new(
            format_args!("git {} gave no two directories: {out}", args.join(" ")),
            1,
        )),
    }
}
