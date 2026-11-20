//! `skills-unmanaged`: lists the entries under `.claude/skills/` that exist on this
//! machine alone.
//!
//! It reads the `.claude/skills/` and apm.lock.yaml of the directory it runs in, which
//! the mise task calling it leaves as the root of the checkout it ran in.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, ExitCode, Stdio};

use crate::failure::{Failure, exit_code};

/// The directory whose entries are listed, relative to the checkout's root.
const SKILLS: &str = ".claude/skills";

const USAGE: &str = "\
Usage: mise run skills:unmanaged

Lists the entries under .claude/skills/ that neither git tracks nor apm.lock.yaml
deploys, one path per line. Such an entry exists on this machine alone: no clone
carries it and no `apm install` restores it. Hidden entries count too. An entry
counts as tracked or deployed when any file under it is.

  --help, -h  print this message
";

/// Prints each unmanaged entry under `.claude/skills/`, one path per line.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args {
        [] => list(),
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

fn list() -> Result<(), Failure> {
    let managed = Managed::read()?;
    let unmanaged: Vec<Vec<u8>> = entries()?
        .into_iter()
        .filter(|path| !managed.contains(path))
        .collect();
    write(&unmanaged).map_err(|error| Failure::writing(&error))
}

/// The paths of the entries under `.claude/skills/`, hidden ones included, in byte
/// order.
fn entries() -> Result<Vec<Vec<u8>>, Failure> {
    let reading = |error: io::Error| Failure::new(format_args!("cannot read {SKILLS}: {error}"), 1);
    let mut paths = fs::read_dir(SKILLS)
        .map_err(reading)?
        .map(|entry| {
            let mut path = format!("{SKILLS}/").into_bytes();
            path.extend_from_slice(entry?.file_name().as_bytes());
            Ok(path)
        })
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(reading)?;
    paths.sort();
    Ok(paths)
}

/// Writes `paths` to stdout, one per line.
fn write(paths: &[Vec<u8>]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    for path in paths {
        out.write_all(path)?;
        out.write_all(b"\n")?;
    }
    out.flush()
}

/// The top-level entries under `.claude/skills/` that git tracks a file under or
/// apm.lock.yaml deploys one under.
struct Managed {
    paths: BTreeSet<Vec<u8>>,
}

impl Managed {
    fn read() -> Result<Self, Failure> {
        let tracked = output(
            Command::new("git").args(["ls-files", "-z", "--", SKILLS]),
            format_args!("git could not list {SKILLS}"),
        )?;
        let deployed = output(
            Command::new("yq").args([".dependencies[].deployed_files[]", "apm.lock.yaml"]),
            format_args!("yq could not read apm.lock.yaml"),
        )?;
        let paths = tracked
            .split(|&byte| byte == 0)
            .chain(deployed.split(|&byte| byte == b'\n'))
            .filter(|path| !path.is_empty())
            .map(top_level)
            .collect();
        Ok(Managed { paths })
    }

    fn contains(&self, path: &[u8]) -> bool {
        self.paths.contains(path)
    }
}

/// `path` cut to its first three components, which for a path under `.claude/skills/`
/// is the top-level entry holding it, as `cut -d/ -f1-3` cuts it.
fn top_level(path: &[u8]) -> Vec<u8> {
    path.split(|&byte| byte == b'/')
        .take(3)
        .collect::<Vec<_>>()
        .join(&b'/')
}

/// What `command` prints, its stderr passed on, or a failure saying what `failing` did
/// where it does not succeed.
fn output(command: &mut Command, failing: fmt::Arguments<'_>) -> Result<Vec<u8>, Failure> {
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| Failure::new(format_args!("cannot run {program}: {error}"), 1))?;
    if !output.status.success() {
        let code = exit_code(output.status);
        return Err(Failure::new(format_args!("{failing} (exit {code})"), code));
    }
    Ok(output.stdout)
}
