//! `setup-dotfiles`: sets this machine up around a dotfiles checkout: dircolors, the
//! zsh startup files, npm's and Hex's release windows (README.md, "Release window"),
//! and rclone's config encryption. Each step is safe to repeat, since `mise bootstrap`
//! runs it on every run.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Args;

use crate::dotfiles_checkout::DotfilesCheckout;
use crate::failure::Failure;

/// The canonical dircolors, which `~/.dircolors` is overwritten with.
const COLORS: &str = "https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS";

/// Set this machine up around the dotfiles checkout it runs in.
///
/// Downloads ~/.dircolors, has ~/.zshenv, ~/.zprofile and ~/.zshrc source the main
/// checkout's zsh fragments, sets npm's and Hex's release windows, and encrypts rclone's
/// config with a password it keeps in the login keychain. Every step is safe to repeat.
/// Run it as `mise run setup:dotfiles`.
#[derive(Debug, Args)]
pub(crate) struct Request {}

impl Request {
    /// Sets the machine up.
    #[expect(
        clippy::unused_self,
        reason = "the command takes no arguments, and every command's entry has one shape"
    )]
    pub(crate) fn run(self) -> Result<(), Failure> {
        set_up()
    }
}

fn set_up() -> Result<(), Failure> {
    let main = DotfilesCheckout::find()?.main()?;
    let home = Home::from_env()?;
    home.fetch_colors()?;
    home.source_fragments(&main)?;
    ReleaseWindows::set()?;
    let rclone = Rclone::from_zsh()?;
    rclone.store_password()?;
    rclone.encrypt_config()
}

/// The user's home directory, which the setup writes into.
struct Home {
    dir: PathBuf,
}

impl Home {
    fn from_env() -> Result<Self, Failure> {
        let dir = std::env::var_os("HOME")
            .filter(|dir| !dir.is_empty())
            .ok_or_else(|| Failure::new(format_args!("[fail] HOME is not set"), 1))?;
        Ok(Home {
            dir: PathBuf::from(dir),
        })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Downloads the colors into `~/.dircolors`, replacing it only once the download
    /// has succeeded, so a failed one on a rerun stops here rather than leaving an
    /// error page where the colors were.
    fn fetch_colors(&self) -> Result<(), Failure> {
        let new = self.path(".dircolors.new");
        let mut curl = Command::new("curl");
        curl.arg("-fsSL").arg("-o").arg(&new).arg(COLORS);
        Change::new(curl).run()?;
        let colors = self.path(".dircolors");
        fs::rename(&new, &colors).map_err(|error| unwritable(&colors, &error))
    }

    /// Has each zsh startup file source its fragment from the checkout at `main`.
    ///
    /// The main checkout's, so an edit there reaches the next shell without rerunning
    /// this command, and not a worktree's, which goes away. A missing fragment (the
    /// checkout moved, or sits on a branch without it) is reported on every shell start
    /// rather than skipped, since unattended shells depend on `.zshenv`'s. A file that
    /// already names its fragment, however it sources it, is left alone, and lines an
    /// installer appends to a file stay. The line starts with a newline in case the
    /// file's last line lacks one, which would otherwise glue the two.
    fn source_fragments(&self, main: &Path) -> Result<(), Failure> {
        for name in ["zshenv", "zprofile", "zshrc"] {
            let file = StartupFile {
                path: self.path(&format!(".{name}")),
            };
            let named = format!("zsh_fragments/{name}.sh");
            if file.names(&named)? {
                continue;
            }
            let fragment = main.join(&named);
            let fragment = fragment.display();
            file.append(&format!(
                "\nif [[ -r \"{fragment}\" ]]; then source \"{fragment}\"; else echo \"{fragment}: missing, not sourced\" >&2; fi\n"
            ))?;
        }
        Ok(())
    }
}

/// A zsh startup file in the home, created where it is missing.
struct StartupFile {
    path: PathBuf,
}

impl StartupFile {
    /// Whether the file holds `text`, a missing one holding nothing. It is read before
    /// any open for writing, so a file that needs no change is never opened for it, as
    /// a read-only one could not be.
    fn names(&self, text: &str) -> Result<bool, Failure> {
        let contents = match fs::read(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                return Err(Failure::new(
                    format_args!("cannot read {}: {error}", self.path.display()),
                    1,
                ));
            }
        };
        Ok(contents
            .windows(text.len())
            .any(|window| window == text.as_bytes()))
    }

    /// Appends `line` in one write: a line cut short would still name its fragment, so
    /// no later run would repair it.
    fn append(&self, line: &str) -> Result<(), Failure> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .map_err(|error| unwritable(&self.path, &error))
    }
}

/// npm's and Hex's release windows, each set in the tool's own user or global config
/// rather than an environment variable, which would outrank a project's own.
struct ReleaseWindows;

impl ReleaseWindows {
    fn set() -> Result<(), Failure> {
        // One key set in place: the file may also hold registry auth.
        let mut npm = Command::new("npm");
        npm.args(["config", "set", "min-release-age=1", "--location=user"]);
        Change::new(npm).run()?;
        // Hex is updated first: its archive is built for one Erlang/OTP, so an older
        // one can fail to load after an upgrade. Twice, since there are two archives:
        // mise points MIX_HOME into the Elixir install, and a shell that has mix on
        // PATH without mise's environment, as an agent's can, falls back to ~/.mix.
        // The config, which every mix reads, is one file for both.
        let mut hex = Command::new("mix");
        hex.args(["local.hex", "--force"]);
        Change::new(hex).run()?;
        let mut home_hex = Command::new("mix");
        home_hex
            .args(["local.hex", "--force"])
            .env_remove("MIX_HOME")
            .env_remove("MIX_ARCHIVES");
        Change::new(home_hex).run()?;
        let mut cooldown = Command::new("mix");
        cooldown.args(["hex.config", "cooldown", "1d"]);
        Change::new(cooldown).run()
    }
}

/// rclone, whose config is encrypted with a password that `RCLONE_PASSWORD_COMMAND` reads
/// from the login keychain.
struct Rclone {
    /// The command that prints the password.
    password_command: String,
}

impl Rclone {
    /// rclone under the password command zsh states: `.zshenv` states it, so it is
    /// written once, and this run's own environment may predate the sourcing line.
    fn from_zsh() -> Result<Self, Failure> {
        let output = Command::new("zsh")
            .args(["-c", "printf %s \"$RCLONE_PASSWORD_COMMAND\""])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| unrunnable("zsh", &error))?;
        if !output.status.success() {
            return Err(Failure::passed_on(output.status));
        }
        let password_command = String::from_utf8_lossy(&output.stdout).into_owned();
        if password_command.is_empty() {
            return Err(Failure::new(
                format_args!("[fail] zsh states no RCLONE_PASSWORD_COMMAND"),
                1,
            ));
        }
        Ok(Rclone { password_command })
    }

    /// `program` with the password command in its environment.
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.env("RCLONE_PASSWORD_COMMAND", &self.password_command);
        command
    }

    /// Stores a random password nobody types where the password command reads it,
    /// unless it reads one already, so the item can be made here, where that command
    /// reads.
    fn store_password(&self) -> Result<(), Failure> {
        // Split on whitespace, as the script's unquoted expansion did; rclone also
        // honours double quotes, which the command `.zshenv` states does not use.
        let mut words = self.password_command.split_whitespace();
        let program = words.next().unwrap_or_default();
        let reads = self
            .command(program)
            .args(words)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if reads {
            return Ok(());
        }
        let password = Self::random_password()?;
        let mut security = Command::new("security");
        security.args([
            "add-generic-password",
            "-a",
            "rclone",
            "-s",
            "config",
            "-w",
            &password,
        ]);
        Change::hiding(
            security,
            "security add-generic-password -a rclone -s config -w <generated>",
        )
        .run()
    }

    /// A random password, 40 bytes in base64.
    fn random_password() -> Result<String, Failure> {
        let output = Command::new("openssl")
            .args(["rand", "-base64", "40"])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| unrunnable("openssl", &error))?;
        if !output.status.success() {
            return Err(Failure::passed_on(output.status));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    }

    /// Encrypts the config with the password, unless it is already.
    ///
    /// `check` fails both on a config not yet encrypted, a missing one included, and on
    /// one encrypted with another password, which `set` cannot open; the header tells
    /// the two apart.
    fn encrypt_config(&self) -> Result<(), Failure> {
        let encrypted = self
            .command("rclone")
            .args(["config", "encryption", "check"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if encrypted {
            return Ok(());
        }
        let config = self.config_path()?;
        let foreign = fs::read(&config).is_ok_and(|contents| {
            contents
                .split(|&byte| byte == b'\n')
                .any(|line| line == b"RCLONE_ENCRYPT_V0:")
        });
        if foreign {
            return Err(Failure::new(
                format_args!(
                    "{} is encrypted with another password; decrypt it with `env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove`, which asks for that password, or without it move the file aside, then rerun",
                    config.display()
                ),
                1,
            ));
        }
        let mut set = self.command("rclone");
        set.args(["config", "encryption", "set"]);
        Change::new(set).run()
    }

    /// The config file `rclone config paths` names.
    fn config_path(&self) -> Result<PathBuf, Failure> {
        let output = self
            .command("rclone")
            .args(["config", "paths"])
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| unrunnable("rclone", &error))?;
        if !output.status.success() {
            return Err(Failure::passed_on(output.status));
        }
        let paths = String::from_utf8_lossy(&output.stdout);
        Ok(PathBuf::from(
            paths
                .lines()
                .find_map(|line| line.strip_prefix("Config file:"))
                .map_or("", str::trim_start),
        ))
    }
}

/// A command that changes the machine, shown on stderr as `+ <command>` before it runs,
/// with any variable it drops from the environment as `env -u <name>` in front.
struct Change {
    command: Command,
    shown: String,
}

impl Change {
    /// `command`, shown with the variables it drops from the environment.
    fn new(command: Command) -> Self {
        let mut words: Vec<&OsStr> = Vec::new();
        let dropped: Vec<&OsStr> = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name)
            .collect();
        if !dropped.is_empty() {
            words.push(OsStr::new("env"));
            for name in dropped {
                words.extend([OsStr::new("-u"), name]);
            }
        }
        words.push(command.get_program());
        words.extend(command.get_args());
        let shown = words
            .iter()
            .map(|word| word.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        Change { command, shown }
    }

    /// `command`, shown as `shown`, for one whose arguments hold a secret.
    fn hiding(command: Command, shown: &str) -> Self {
        Change {
            command,
            shown: shown.to_owned(),
        }
    }

    /// Shows the command and runs it, its output passed on, failing with its exit
    /// code.
    fn run(mut self) -> Result<(), Failure> {
        let _ = writeln!(io::stderr(), "+ {}", self.shown);
        let program = self.command.get_program().to_string_lossy().into_owned();
        let status = self
            .command
            .status()
            .map_err(|error| unrunnable(&program, &error))?;
        if status.success() {
            Ok(())
        } else {
            Err(Failure::passed_on(status))
        }
    }
}

fn unrunnable(program: &str, error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot run {program}: {error}"), 1)
}

fn unwritable(path: &Path, error: &io::Error) -> Failure {
    Failure::new(format_args!("cannot write {}: {error}", path.display()), 1)
}
