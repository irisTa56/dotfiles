//! `push-gist-skill`: pushes local edits to a gist-sourced skill back to its gist, then
//! moves the pin to them.
//!
//! APM deploys the skill from apm.yml's gist dependency (aliased to the skill's name)
//! and restores the pinned content on every `apm install`, so a local edit is lost
//! unless it is pushed here and the lockfile advanced past it.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use tempfile::TempDir;

use crate::failure::Failure;
use crate::git::{git, read};

/// The manifest of dotfiles-local, which marks a dotfiles checkout.
const MANIFEST: &str = "rust/crates/local/Cargo.toml";

const USAGE: &str = "\
Usage: mise run skills:push <name>

Pushes the local .claude/skills/<name>/SKILL.md of a gist-sourced skill back to its
gist, then has `apm update` pin the pushed commit in apm.lock.yaml, so the next
`apm install` restores the edit rather than reverting it. <name> is the alias of the
skill's gist dependency in apm.yml. The commit goes on top of the pinned revision,
so a gist that has moved past it rejects the push. Run it in a dotfiles checkout,
whose skill and lockfile it acts on; gh supplies the credential for the push.

  --help, -h  print this message
";

/// Pushes the skill `args` names.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args {
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        [name] => push(name),
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => failure.report(),
    }
}

/// Commits the local SKILL.md on top of the pinned revision of its gist, pushes it,
/// and has apm pin the pushed commit.
fn push(name: &str) -> Result<(), Failure> {
    // The name becomes a path component.
    if matches!(name, "" | "." | "..") || name.contains('/') {
        let shown = if name.is_empty() { "<empty>" } else { name };
        return Err(Failure::new(
            format_args!("[fail] {shown}: invalid skill name"),
            1,
        ));
    }
    let checkout = Checkout::find()?;
    let repo = checkout.locked(name, "repo_url")?;
    if !is_gist_repo(&repo) {
        return Err(Failure::new(
            format_args!("[fail] {name}: no gist dependency by that alias in apm.lock.yaml"),
            1,
        ));
    }
    let file = checkout.skill(name);
    if !file.is_file() {
        return Err(Failure::new(
            format_args!(
                "[fail] {name}: no local SKILL.md at .claude/skills/{name}/SKILL.md (run 'apm install' first)"
            ),
            1,
        ));
    }
    let gist = Gist::clone(&repo)?;
    gist.reset(&checkout.locked(name, "resolved_commit")?)?;
    let pushed = gist.commit_and_push(&file, name)?;
    let _ = writeln!(io::stdout(), "[ok] {name} -> gist {repo} at {pushed}");
    checkout.update(name)?;
    let pinned = checkout.locked(name, "resolved_commit")?;
    if pinned != pushed {
        return Err(Failure::new(
            format_args!(
                "[fail] {name}: pushed {pushed} but apm.lock.yaml pins {pinned}; rerun 'apm update --yes {name}'"
            ),
            1,
        ));
    }
    let _ = writeln!(
        io::stdout(),
        "[note] commit apm.lock.yaml and land it on main, or an install from main restores the old copy"
    );
    Ok(())
}

/// The dotfiles checkout the command runs in, a worktree as much as the main one.
struct Checkout {
    root: PathBuf,
}

impl Checkout {
    /// The checkout holding the directory the command runs in, which has to be a
    /// dotfiles one: in another repository the command would push that repository's
    /// gist. A checkout counts as dotfiles where it holds dotfiles-local's own source.
    fn find() -> Result<Self, Failure> {
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
        Ok(Checkout { root })
    }

    /// The local SKILL.md of the skill `name`.
    fn skill(&self, name: &str) -> PathBuf {
        self.root.join(".claude/skills").join(name).join("SKILL.md")
    }

    /// The `field` of the skill `name`'s gist dependency in apm.lock.yaml.
    fn locked(&self, name: &str, field: &str) -> Result<String, Failure> {
        let output = Command::new("yq")
            .current_dir(&self.root)
            .env("N", name)
            .args([
                "-r",
                &format!(
                    ".dependencies[] | select(.name == strenv(N) and .host == \"gist.github.com\") | .{field}"
                ),
                "apm.lock.yaml",
            ])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| Failure::new(format_args!("cannot run yq: {error}"), 1))?;
        if !output.status.success() {
            return Err(Failure::passed_on(output.status));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end_matches('\n')
            .to_owned())
    }

    /// Has apm pin the gist's head for the skill `name`.
    fn update(&self, name: &str) -> Result<(), Failure> {
        let status = Command::new("apm")
            .current_dir(&self.root)
            .args(["update", "--yes", name])
            .status()
            .map_err(|error| Failure::new(format_args!("cannot run apm: {error}"), 1))?;
        if status.success() {
            Ok(())
        } else {
            Err(Failure::passed_on(status))
        }
    }
}

/// A clone of the skill's gist in a directory removed with it.
struct Gist {
    dir: TempDir,
}

impl Gist {
    /// Clones the gist `repo`, `<owner>/<gist_id>`.
    fn clone(repo: &str) -> Result<Self, Failure> {
        let dir = TempDir::new()
            .map_err(|error| Failure::new(format_args!("cannot create a directory: {error}"), 1))?;
        let gist = Gist { dir };
        git(&[
            "clone",
            "-q",
            &format!("https://gist.github.com/{repo}.git"),
            gist.path()?,
        ])?;
        Ok(gist)
    }

    fn path(&self) -> Result<&str, Failure> {
        self.dir.path().to_str().ok_or_else(|| {
            Failure::new(
                format_args!("{}: not a UTF-8 path", self.dir.path().display()),
                1,
            )
        })
    }

    /// Moves the clone to `commit`, the pinned revision.
    fn reset(&self, commit: &str) -> Result<(), Failure> {
        git(&["-C", self.path()?, "reset", "-q", "--hard", commit])
    }

    /// Commits `file` as the gist's SKILL.md, pushes it, and returns the pushed commit.
    fn commit_and_push(&self, file: &Path, name: &str) -> Result<String, Failure> {
        let dir = self.path()?;
        fs::copy(file, self.dir.path().join("SKILL.md"))
            .map_err(|error| Failure::new(format_args!("cannot copy: {error}"), 1))?;
        git(&["-C", dir, "commit", "-qam", &format!("Update {name}")])?;
        // gh supplies the credential for this push alone; the empty helper first drops
        // the configured ones, so none of them stores gh's token.
        git(&[
            "-C",
            dir,
            "-c",
            "credential.helper=",
            "-c",
            "credential.helper=!gh auth git-credential",
            "push",
            "-q",
            "origin",
            "HEAD",
        ])
        .map_err(|_| {
            // The local SKILL.md is now the edit's only copy, and `apm update` would
            // overwrite it.
            Failure::new(
                format_args!(
                    "[fail] {name}: push refused (see git's message above); if the gist moved past the pin, copy .claude/skills/{name}/SKILL.md aside, run 'apm update --yes {name}', redo the edit on top and rerun"
                ),
                1,
            )
        })?;
        read(&["-C", dir, "rev-parse", "HEAD"])
    }
}

/// Whether `repo` is `<owner>/<gist_id>`, as a gist dependency's `repo_url` is.
fn is_gist_repo(repo: &str) -> bool {
    repo.split_once('/').is_some_and(|(owner, id)| {
        !owner.is_empty()
            && !id.is_empty()
            && id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
