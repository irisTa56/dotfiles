//! `fnox-add-secret`: stores an API key in the keychain for this repository, through
//! fnox; the README's Secrets section is the convention it carries out.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::failure::Failure;
use crate::git::read;

/// The fnox config the command declares the provider in and stores the key through,
/// git-ignored, so a worktree has none and fnox finds the main checkout's from one
/// under it.
const CONFIG: &str = "fnox.local.toml";

/// The copy of the config the declaration goes into first.
const DRAFT: &str = "fnox.local.toml.new";

/// The line telling an agent in the repository how secrets reach a command.
const NOTE: &str = "- Secrets here come from the macOS keychain through fnox (`fnox.local.toml`), not `.env`: run a command that needs one as `fnox exec -- <command>`.";

const USAGE: &str = "\
Usage: fnox-add-secret <NAME>

Stores the key <NAME> in the macOS keychain for this repository through fnox, which
prompts for its value. The main checkout's fnox.local.toml gets a keychain provider
named after the repository where it lacks one. The line on running a command under
`fnox exec` goes into a new CLAUDE.local.md there, or is printed for you to place in
an existing one that lacks it.

  --help, -h  print this message
";

/// Stores the key named by the one name `args` holds, or, given `--help`, prints how
/// it is called.
pub(crate) fn run(args: &[String]) -> Result<(), Failure> {
    match args {
        [flag] if flag == "--help" || flag == "-h" => {
            let _ = writeln!(io::stdout(), "{USAGE}");
            Ok(())
        }
        // A name starting with `-` would reach `fnox set` as an option.
        [name] if !name.starts_with('-') => add(name),
        _ => Err(Failure::new(format_args!("{USAGE}"), 2)),
    }
}

/// Declares the repository's provider where it is missing and stores the key `name`
/// through it.
fn add(name: &str) -> Result<(), Failure> {
    let checkout = Checkout::main()?;
    if !checkout.has_provider()? {
        checkout.declare_provider()?;
    }
    checkout.store(name)?;
    checkout.note()
}

/// The main checkout of the repository the command runs in, not a worktree.
struct Checkout {
    root: PathBuf,
}

impl Checkout {
    /// The main checkout, the first worktree git lists.
    fn main() -> Result<Self, Failure> {
        let list = read(&["worktree", "list", "--porcelain"])?;
        let root = list
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("worktree "))
            .unwrap_or_default();
        Ok(Checkout {
            root: PathBuf::from(root),
        })
    }

    /// The provider's name, the repository's.
    fn provider(&self) -> String {
        self.root
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    /// fnox with `args`, run in the checkout.
    fn fnox(&self, args: &[&str]) -> Command {
        let mut command = Command::new("fnox");
        command.current_dir(&self.root).args(args);
        command
    }

    fn config(&self) -> PathBuf {
        self.root.join(CONFIG)
    }

    /// Whether fnox lists the repository's provider among those it loads with the
    /// config, created empty where it is missing.
    ///
    /// The provider is named after the repository: fnox lists every provider it loads,
    /// from parent directories and the global config too, and one of theirs would store
    /// the key under their keychain service rather than this one's. fnox does the
    /// parsing, so a declaration in any TOML form counts, and a file it cannot parse
    /// stops the command.
    fn has_provider(&self) -> Result<bool, Failure> {
        let path = self.config();
        // Following a symlink, so one whose target is not there yet gets it created.
        if !path.exists() {
            fs::File::create(&path).map_err(|error| unwritable(&path, &error))?;
        }
        let output = self
            .fnox(&["provider", "list", "-c", CONFIG])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| unrunnable(&error))?;
        if !output.status.success() {
            return Err(Failure::passed_on(output.status));
        }
        let provider = self.provider();
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == provider))
    }

    /// Appends a keychain provider named after the repository to the config, on a
    /// line of its own.
    ///
    /// Some TOML shapes cannot take an appended table (an inline `providers = {…}`),
    /// so the declaration goes into a copy, which keeps the config's mode, that
    /// replaces the config only once fnox parses it.
    fn declare_provider(&self) -> Result<(), Failure> {
        let provider = self.provider();
        let path = self.config();
        let draft = Draft(self.root.join(DRAFT));
        fs::copy(&path, &draft.0).map_err(|error| unwritable(&draft.0, &error))?;
        let ends_open = fs::read(&path)
            .map_err(|error| unreadable(&path, &error))?
            .last()
            .is_some_and(|&byte| byte != b'\n');
        let declaration = format!(
            "{}[providers.\"{provider}\"]\ntype = \"keychain\"\nservice = \"{provider}\"\n\n",
            if ends_open { "\n" } else { "" }
        );
        OpenOptions::new()
            .append(true)
            .open(&draft.0)
            .and_then(|mut file| file.write_all(declaration.as_bytes()))
            .map_err(|error| unwritable(&draft.0, &error))?;
        let parsed = self
            .fnox(&["provider", "list", "-c", DRAFT])
            .stdout(Stdio::null())
            .status()
            .map_err(|error| unrunnable(&error))?;
        if !parsed.success() {
            return Err(Failure::new(
                format_args!(
                    "{CONFIG} cannot take the declaration as appended; left unchanged, so add a keychain provider named {provider} by hand"
                ),
                1,
            ));
        }
        fs::rename(&draft.0, &path).map_err(|error| unwritable(&path, &error))
    }

    /// Has fnox store the key `name` with the repository's provider, `-c` since
    /// `fnox set` writes to fnox.toml where the directory has one.
    fn store(&self, name: &str) -> Result<(), Failure> {
        let status = self
            .fnox(&["set", "-c", CONFIG, name, "--provider", &self.provider()])
            .status()
            .map_err(|error| unrunnable(&error))?;
        if status.success() {
            Ok(())
        } else {
            Err(Failure::passed_on(status))
        }
    }

    /// Writes the line on how secrets reach a command into a new CLAUDE.local.md, and
    /// where one exists without it, tells the user to add it: an existing file has a
    /// structure of its own, and where the line belongs in it is a call this command
    /// cannot make.
    fn note(&self) -> Result<(), Failure> {
        let path = self.root.join("CLAUDE.local.md");
        // Following a symlink, so one whose target is not there yet is written through.
        if !path.exists() {
            return fs::write(&path, format!("{NOTE}\n"))
                .map_err(|error| unwritable(&path, &error));
        }
        let contents = fs::read(&path).map_err(|error| unreadable(&path, &error))?;
        if !String::from_utf8_lossy(&contents).contains("fnox exec") {
            let _ = writeln!(
                io::stderr(),
                "Add this line where it fits in {}:\n{NOTE}",
                path.display()
            );
        }
        Ok(())
    }
}

/// The copy of the config at its path, removed where it has not replaced the config
/// by the time it goes out of scope.
struct Draft(PathBuf);

impl Drop for Draft {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn unwritable(path: &Path, error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot write {}: {error}", path.display()), 1)
}

fn unreadable(path: &Path, error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot read {}: {error}", path.display()), 1)
}

fn unrunnable(error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot run fnox: {error}"), 1)
}
