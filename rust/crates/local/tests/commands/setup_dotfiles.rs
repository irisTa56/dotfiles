//! `setup-dotfiles`: sets up this machine's shell startup files, dircolors, release
//! windows and rclone config encryption, run with the real git, zsh and openssl against
//! a checkout and a home of the test's own. Fakes stand in for curl, npm, mix, rclone
//! and security, so nothing reaches the network, the keychain or the machine's own
//! config; each records how it was called in one log, in order.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A dotfiles checkout of the test's own, a home, a directory holding the fakes and the
/// records they leave, none of which the machine's git config reaches.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// A fresh sandbox named after `test`: a checkout holding one commit that marks it
    /// as dotfiles, with a `.zshenv` fragment that states the keychain command, and an
    /// empty home.
    fn new(test: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("setup-dotfiles")
            .join(test);
        let _ = fs::remove_dir_all(&root);
        for dir in ["bin", "home", "repo"] {
            fs::create_dir_all(root.join(dir)).expect("the sandbox is created");
        }
        let sandbox = Sandbox { root };
        sandbox.write(
            "rust/crates/local/Cargo.toml",
            "[package]\nname = \"dotfiles-local\"\n",
        );
        sandbox.write(
            "zsh_fragments/zshenv.sh",
            "export RCLONE_PASSWORD_COMMAND=\"security find-generic-password -a rclone -s config -w\"\n",
        );
        sandbox.git(&sandbox.repo(), &["init", "--quiet"]);
        sandbox.git(&sandbox.repo(), &["add", "."]);
        sandbox.git(
            &sandbox.repo(),
            &["commit", "--quiet", "--message", "first"],
        );
        sandbox.fakes();
        sandbox
    }

    /// Writes the fakes into the sandbox's PATH. Each appends its name and arguments to
    /// `calls`, and exits with the code in `<name>-code`, if any.
    ///
    /// - curl writes `fetched colors` to the path after `-o`.
    /// - mix also records the `MIX_HOME` and `MIX_ARCHIVES` that reached it.
    /// - security keeps the keychain item in `keychain`: `find-generic-password` prints
    ///   it or fails where there is none, and `add-generic-password` stores what follows
    ///   `-w`, recording `-w <password>` in `calls` as given.
    /// - rclone's `config encryption check` succeeds where `encrypted` exists,
    ///   `config paths` names `rclone.conf` in the sandbox, and `config encryption set`
    ///   records the password command it was given.
    fn fakes(&self) {
        let prologue = r#"#!/bin/sh
printf '%s\n' "$(basename "$0") $*" >> "$SANDBOX/calls"
code=$(cat "$SANDBOX/$(basename "$0")-code" 2>/dev/null || echo 0)
[ "$code" = 0 ] || exit "$code"
"#;
        self.fake(
            "curl",
            &format!(
                r#"{prologue}while [ "$1" != -o ]; do shift; done
printf 'fetched colors\n' > "$2"
"#
            ),
        );
        self.fake("npm", prologue);
        self.fake(
            "mix",
            &format!(
                "{prologue}printf 'MIX_HOME=%s MIX_ARCHIVES=%s\\n' \"${{MIX_HOME-unset}}\" \"${{MIX_ARCHIVES-unset}}\" >> \"$SANDBOX/calls\"\n"
            ),
        );
        self.fake(
            "security",
            &format!(
                r#"{prologue}case "$1" in
find-generic-password) cat "$SANDBOX/keychain" 2>/dev/null || exit 44 ;;
add-generic-password)
  while [ "$1" != -w ]; do shift; done
  printf '%s' "$2" > "$SANDBOX/keychain"
  ;;
esac
"#
            ),
        );
        self.fake(
            "rclone",
            &format!(
                r#"{prologue}case "$*" in
"config encryption check") [ -e "$SANDBOX/encrypted" ] ;;
"config paths") printf 'Config file: %s\nCache dir: /nowhere\n' "$SANDBOX/rclone.conf" ;;
"config encryption set") printf 'RCLONE_PASSWORD_COMMAND=%s\n' "$RCLONE_PASSWORD_COMMAND" >> "$SANDBOX/calls" ;;
esac
"#
            ),
        );
    }

    fn fake(&self, name: &str, body: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, body).expect("the fake is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("the fake is made executable");
    }

    /// The fakes' calls, one a line, in order, or the empty string where none was made.
    fn calls(&self) -> String {
        fs::read_to_string(self.root.join("calls")).unwrap_or_default()
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The names of what the home holds, sorted.
    fn home_entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.home())
            .expect("the home is read")
            .map(|entry| {
                entry
                    .expect("the entry is read")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    /// Writes `contents` to the checkout's `path`, creating its directories.
    fn write(&self, path: &str, contents: &str) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("the directories are created");
        fs::write(path, contents).expect("the file is written");
    }

    /// Runs git with `args` in `dir`.
    fn git(&self, dir: &Path, args: &[&str]) {
        let status = self
            .command("git", dir)
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// `program` run in `dir`, with what a git hook sets in the environment and the
    /// keychain command taken away, the machine's git config and zsh startup files left
    /// unread, the sandbox's home, and the fakes first on PATH.
    fn command(&self, program: &str, dir: &Path) -> Command {
        let mut command = Command::new(program);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let path = std::env::join_paths(
            std::iter::once(self.root.join("bin")).chain(std::env::split_paths(&inherited)),
        )
        .expect("PATH is joined");
        command
            .current_dir(dir)
            .env_remove("RCLONE_PASSWORD_COMMAND")
            .env_remove("ZDOTDIR")
            .env("PATH", path)
            .env("HOME", self.home())
            .env("SANDBOX", &self.root)
            .env("MIX_HOME", "/elixir/mix")
            .env("MIX_ARCHIVES", "/elixir/mix/archives")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.root)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com");
        command
    }

    /// Runs `setup-dotfiles` with `args` in `dir`.
    fn run(&self, dir: &Path, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_dotfiles-local"), dir)
            .arg("setup-dotfiles")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

fn stderr(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).into_owned()
}

/// What the run said on stderr besides showing the commands it ran.
fn said(run: &Output) -> String {
    stderr(run)
        .lines()
        .filter(|line| !line.starts_with("+ "))
        .flat_map(|line| [line, "\n"])
        .collect()
}

// Outside a dotfiles checkout the command would point another repository's files at
// the shell, so it fails before it writes or runs anything.
#[test]
fn fails_without_writing_outside_a_dotfiles_checkout() {
    let sandbox = Sandbox::new("elsewhere");
    fs::remove_file(sandbox.repo().join("rust/crates/local/Cargo.toml"))
        .expect("the marker is removed");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(
        (run.status.code(), stderr(&run)),
        (
            Some(1),
            format!(
                "[fail] {} is not a dotfiles checkout: no rust/crates/local/Cargo.toml naming dotfiles-local\n",
                sandbox.repo().display()
            )
        )
    );
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.home_entries(), Vec::<String>::new());
}

impl Sandbox {
    /// What the home's `name` holds, or `None` where it has no such file.
    fn home_file(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.home().join(name)).ok()
    }
}

/// The URL the colors come from.
const COLORS: &str = "https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS";

// The colors are downloaded beside ~/.dircolors and then moved over it.
#[test]
fn downloads_the_colors_into_dircolors() {
    let sandbox = Sandbox::new("colors");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.home_file(".dircolors").as_deref(),
        Some("fetched colors\n")
    );
    assert_eq!(sandbox.home_file(".dircolors.new"), None);
    assert_eq!(
        sandbox.calls().lines().next(),
        Some(
            format!(
                "curl -fsSL -o {}/.dircolors.new {COLORS}",
                sandbox.home().display()
            )
            .as_str()
        )
    );
}

/// The line that sources the fragment `name` of the checkout at `root`.
fn sourcing(root: &Path, name: &str) -> String {
    let fragment = format!("{}/zsh_fragments/{name}.sh", root.display());
    format!(
        "\nif [[ -r \"{fragment}\" ]]; then source \"{fragment}\"; else echo \"{fragment}: missing, not sourced\" >&2; fi\n"
    )
}

// Each startup file, created where missing, gets the line that sources its fragment,
// the line starting with a newline in case the file's last line lacks one.
#[test]
fn appends_the_sourcing_lines_to_the_startup_files() {
    let sandbox = Sandbox::new("startup");
    fs::write(sandbox.home().join(".zshrc"), "eval \"$(installer init)\"")
        .expect("the file is written");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let repo = sandbox.repo();
    assert_eq!(
        [".zshenv", ".zprofile", ".zshrc"].map(|name| sandbox.home_file(name)),
        [
            Some(sourcing(&repo, "zshenv")),
            Some(sourcing(&repo, "zprofile")),
            Some(format!(
                "eval \"$(installer init)\"{}",
                sourcing(&repo, "zshrc")
            )),
        ]
    );
}

// A file that already names its fragment, however it sources it, is left alone, so a
// rerun adds nothing.
#[test]
fn leaves_a_startup_file_that_names_its_fragment() {
    let sandbox = Sandbox::new("rerun");
    let own = "source ~/elsewhere/zsh_fragments/zshrc.sh\n";
    fs::write(sandbox.home().join(".zshrc"), own).expect("the file is written");

    let first = sandbox.run(&sandbox.repo(), &[]);
    let second = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    assert_eq!(second.status.code(), Some(0), "{}", stderr(&second));
    assert_eq!(sandbox.home_file(".zshrc").as_deref(), Some(own));
    assert_eq!(
        sandbox.home_file(".zshenv"),
        Some(sourcing(&sandbox.repo(), "zshenv"))
    );
}

// Run from a worktree, the lines still point at the main checkout's fragments, since a
// worktree goes away.
#[test]
fn points_a_worktrees_run_at_the_main_checkouts_fragments() {
    let sandbox = Sandbox::new("worktree");
    let worktree = sandbox.root.join("worktree");
    sandbox.git(
        &sandbox.repo(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "feature",
            worktree.to_str().expect("UTF-8"),
        ],
    );

    let run = sandbox.run(&worktree, &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.home_file(".zshrc"),
        Some(sourcing(&sandbox.repo(), "zshrc"))
    );
}

// npm's and Hex's release windows are set in their user config, Hex updated first in
// both of its archives: the one mise's environment names, and the one in ~/.mix that a
// shell without it falls back to.
#[test]
fn sets_the_release_windows_after_updating_hex_in_both_archives() {
    let sandbox = Sandbox::new("windows");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.calls().lines().skip(1).take(7).collect::<Vec<_>>(),
        [
            "npm config set min-release-age=1 --location=user",
            "mix local.hex --force",
            "MIX_HOME=/elixir/mix MIX_ARCHIVES=/elixir/mix/archives",
            "mix local.hex --force",
            "MIX_HOME=unset MIX_ARCHIVES=unset",
            "mix hex.config cooldown 1d",
            "MIX_HOME=/elixir/mix MIX_ARCHIVES=/elixir/mix/archives",
        ]
    );
}

impl Sandbox {
    /// The fakes' calls after the release windows', the password the keychain holds, if
    /// any, shown as `<password>`.
    fn rclone_calls(&self) -> Vec<String> {
        let password = self.password();
        self.calls()
            .lines()
            .skip(8)
            .map(|line| match &password {
                Some(password) => line.replace(password.as_str(), "<password>"),
                None => line.to_owned(),
            })
            .collect()
    }

    /// The password the keychain item holds, or `None` where there is no item.
    fn password(&self) -> Option<String> {
        fs::read_to_string(self.root.join("keychain")).ok()
    }
}

/// The keychain command `.zshenv`'s fragment states.
const KEYCHAIN: &str = "security find-generic-password -a rclone -s config -w";

// With no keychain item and no encrypted config, a random password is stored where the
// command `.zshenv` states reads it, and the config is encrypted under that command.
#[test]
fn stores_a_random_password_and_encrypts_the_config_with_it() {
    let sandbox = Sandbox::new("rclone");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let password = sandbox.password().expect("the item is stored");
    assert_eq!(password.len(), 56, "{password}");
    assert_eq!(
        sandbox.rclone_calls(),
        [
            KEYCHAIN.to_owned(),
            "security add-generic-password -a rclone -s config -w <password>".to_owned(),
            "rclone config encryption check".to_owned(),
            "rclone config paths".to_owned(),
            "rclone config encryption set".to_owned(),
            format!("RCLONE_PASSWORD_COMMAND={KEYCHAIN}"),
        ]
    );
}

// A rerun on a machine already set up keeps the stored password and the encrypted
// config as they are, and the password the keychain command prints appears on neither
// stream.
#[test]
fn keeps_a_stored_password_and_an_encrypted_config() {
    let sandbox = Sandbox::new("encrypted");
    let stored = "stored-password-7f3a";
    fs::write(sandbox.root.join("keychain"), stored).expect("the item is stored");
    fs::write(sandbox.root.join("encrypted"), "").expect("the config is marked");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(sandbox.password().as_deref(), Some(stored));
    let output = format!("{}{}", String::from_utf8_lossy(&run.stdout), stderr(&run));
    assert!(!output.contains(stored), "{output}");
    assert_eq!(
        sandbox.rclone_calls(),
        [KEYCHAIN, "rclone config encryption check"]
    );
}

// A config encrypted with another password cannot be opened with the stored one, so
// the command stops and says how to recover rather than encrypting it again.
#[test]
fn stops_on_a_config_encrypted_with_another_password() {
    let sandbox = Sandbox::new("foreign");
    let config = sandbox.root.join("rclone.conf");
    fs::write(
        &config,
        "# Encrypted rclone configuration File\n\nRCLONE_ENCRYPT_V0:\nabc\n",
    )
    .expect("the config is written");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(
        (run.status.code(), said(&run)),
        (
            Some(1),
            format!(
                "{} is encrypted with another password; decrypt it with `env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove`, which asks for that password, or without it move the file aside, then rerun\n",
                config.display()
            )
        )
    );
    assert_eq!(
        sandbox.rclone_calls().last().map(String::as_str),
        Some("rclone config paths")
    );
}

// Without the keychain command from `.zshenv` there is nowhere to store the password,
// so the command stops before rclone.
#[test]
fn stops_where_zsh_states_no_keychain_command() {
    let sandbox = Sandbox::new("no-command");
    sandbox.write("zsh_fragments/zshenv.sh", "");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(
        (run.status.code(), said(&run)),
        (
            Some(1),
            "[fail] zsh states no RCLONE_PASSWORD_COMMAND\n".to_owned()
        )
    );
    assert_eq!(sandbox.rclone_calls(), Vec::<String>::new());
}

// Each command that changes something is shown on stderr before it runs, but the
// generated password appears on neither stream.
#[test]
fn shows_each_command_but_never_the_password() {
    let sandbox = Sandbox::new("shown");

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let password = sandbox.password().expect("the item is stored");
    let output = format!("{}{}", String::from_utf8_lossy(&run.stdout), stderr(&run));
    assert!(!output.contains(&password), "{output}");
    assert_eq!(
        stderr(&run)
            .lines()
            .filter(|line| line.starts_with("+ "))
            .collect::<Vec<_>>(),
        [
            format!(
                "+ curl -fsSL -o {}/.dircolors.new {COLORS}",
                sandbox.home().display()
            )
            .as_str(),
            "+ npm config set min-release-age=1 --location=user",
            "+ mix local.hex --force",
            "+ env -u MIX_ARCHIVES -u MIX_HOME mix local.hex --force",
            "+ mix hex.config cooldown 1d",
            "+ security add-generic-password -a rclone -s config -w <generated>",
            "+ rclone config encryption set",
        ]
    );
}

impl Sandbox {
    /// Makes the fake `name` exit with `code`.
    fn fail(&self, name: &str, code: u8) {
        fs::write(self.root.join(format!("{name}-code")), code.to_string())
            .expect("the code is written");
    }
}

// A failed download leaves the colors already there, and stops the run with curl's
// exit code before anything else changes.
#[test]
fn stops_on_a_failed_download_keeping_the_colors() {
    let sandbox = Sandbox::new("offline");
    fs::write(sandbox.home().join(".dircolors"), "old colors\n").expect("the colors are written");
    sandbox.fail("curl", 22);

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(22), "{}", stderr(&run));
    assert_eq!(
        sandbox.home_file(".dircolors").as_deref(),
        Some("old colors\n")
    );
    assert_eq!(sandbox.home_entries(), [".dircolors"]);
    assert_eq!(sandbox.calls().lines().count(), 1, "{}", sandbox.calls());
}

// A step that fails stops the run with its exit code, before the steps after it.
#[test]
fn stops_at_a_failed_step_with_its_exit_code() {
    let sandbox = Sandbox::new("npm-fails");
    sandbox.fail("npm", 3);

    let run = sandbox.run(&sandbox.repo(), &[]);

    assert_eq!(run.status.code(), Some(3), "{}", stderr(&run));
    assert_eq!(
        sandbox.calls().lines().last(),
        Some("npm config set min-release-age=1 --location=user")
    );
}

// The command takes no arguments: `--help` prints how it is run, and anything else is
// told so.
#[test]
fn prints_usage_on_help_and_fails_with_it_on_arguments() {
    let sandbox = Sandbox::new("usage");

    let help = sandbox.run(&sandbox.repo(), &["--help"]);
    let extra = sandbox.run(&sandbox.repo(), &["now"]);

    assert_eq!(help.status.code(), Some(0), "{}", stderr(&help));
    let usage = String::from_utf8_lossy(&help.stdout).into_owned();
    assert!(
        usage.starts_with("Usage: mise run setup:dotfiles\n"),
        "{usage}"
    );
    assert_eq!((extra.status.code(), stderr(&extra)), (Some(2), usage));
    assert_eq!(sandbox.calls(), "");
    assert_eq!(sandbox.home_entries(), Vec::<String>::new());
}
