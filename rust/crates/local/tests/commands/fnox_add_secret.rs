//! `fnox-add-secret`: stores an API key in the keychain for a repository through fnox,
//! run with the real git against a repository of the test's own and a fake fnox, which
//! records how it was called and stands in for the keychain.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::support::{DOTFILES_LOCAL, IDENTITY, Sandbox, isolated, output_with_input};

/// A repository named `repo` to run in, a directory holding the fakes, and the records
/// the fakes leave, none of which the machine's git config reaches.
struct Fixture {
    sandbox: Sandbox,
}

impl Fixture {
    /// A fresh sandbox named after `test`, its repository holding one commit.
    fn new(test: &str) -> Self {
        let fixture = Fixture {
            sandbox: Sandbox::new("fnox-add-secret", test, &["home", "repo"]),
        };
        fixture.fake_fnox();
        fixture.git(&["init", "--quiet"]);
        fixture.git(&["commit", "--quiet", "--allow-empty", "--message", "first"]);
        fixture
    }

    /// Writes a fake fnox into the sandbox's PATH, which records each call's arguments
    /// on a line of `calls`. `provider list -c <file>` prints the
    /// providers `<file>` declares as `[providers."<name>"]` tables, and fails where the
    /// file holds the word `unparseable` or, as TOML forbids, defines `providers` both
    /// inline and as a table; `set` records its working directory, its
    /// arguments and what it read from stdin, and exits with the code in `set-code`, if
    /// any.
    fn fake_fnox(&self) {
        let body = r#"echo "$*" >> "$SANDBOX/calls"
case "$1 $2" in
"provider list")
  if grep -q unparseable "$4" ||
    { grep -q '^providers = ' "$4" && grep -q '^\[providers\.' "$4"; }; then
    echo "fnox: cannot parse $4" >&2
    exit 1
  fi
  sed -n 's/^\[providers\."\(.*\)"\]$/\1/p' "$4"
  ;;
"set "*)
  { pwd; printf '%s\n' "$@"; } > "$SANDBOX/set-call"
  cat > "$SANDBOX/set-stdin"
  exit "$(cat "$SANDBOX/set-code" 2>/dev/null || echo 0)"
  ;;
esac"#;
        self.sandbox.fake("fnox", body);
    }

    fn root(&self) -> &Path {
        self.sandbox.root()
    }

    /// The working directory and arguments `fnox set` was called with, one a line, or
    /// `None` where it was not called.
    fn set_call(&self) -> Option<String> {
        fs::read_to_string(self.root().join("set-call")).ok()
    }

    /// The arguments of every call fnox got, one call a line, or `None` where it got
    /// none.
    fn fnox_calls(&self) -> Option<String> {
        fs::read_to_string(self.root().join("calls")).ok()
    }

    /// Makes `fnox set` exit with `code`.
    fn fail_set(&self, code: u8) {
        fs::write(self.root().join("set-code"), code.to_string()).expect("the code is written");
    }

    /// What the repository's `name` holds, or `None` where it has no such file.
    fn file(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.repo().join(name)).ok()
    }

    /// Writes `contents` to the repository's `name`.
    fn write(&self, name: &str, contents: &str) {
        fs::write(self.repo().join(name), contents).expect("the file is written");
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    /// Runs git with `args` in the repository.
    fn git(&self, args: &[&str]) {
        let status = self
            .command("git", &self.repo())
            .args(args)
            .status()
            .expect("git starts");
        assert!(status.success(), "git {args:?} failed");
    }

    /// `program` run in `dir`, with what a git hook sets in the environment taken away,
    /// the machine's git config left unread, the sandbox's home, and the fakes first on
    /// PATH.
    fn command(&self, program: &str, dir: &Path) -> Command {
        let mut command = isolated(program, dir, self.root());
        command
            .env("PATH", self.sandbox.path_before_inherited())
            .env("HOME", self.root().join("home"))
            .env("SANDBOX", self.root())
            .envs(IDENTITY);
        command
    }

    /// Runs `fnox-add-secret` with `args` in `dir`.
    fn run(&self, dir: &Path, args: &[&str]) -> Output {
        self.command(DOTFILES_LOCAL, dir)
            .arg("fnox-add-secret")
            .args(args)
            .output()
            .expect("dotfiles-local starts")
    }
}

impl Fixture {
    /// Runs `fnox-add-secret` with `args` in the repository, `input` on its stdin.
    fn run_with_input(&self, args: &[&str], input: &str) -> Output {
        let mut command = self.command(DOTFILES_LOCAL, &self.repo());
        command.arg("fnox-add-secret").args(args);
        output_with_input(command, input.as_bytes())
    }

    /// What `fnox set` read from stdin, or `None` where it was not called.
    fn set_stdin(&self) -> Option<String> {
        fs::read_to_string(self.root().join("set-stdin")).ok()
    }
}

fn stderr(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).into_owned()
}

// The command takes exactly one name, not one starting with `-`, which would reach fnox
// as an option, and is told how it is called otherwise, having written nothing and run
// no fnox.
#[test]
fn fails_with_usage_unless_given_one_name() {
    let sandbox = Fixture::new("usage");
    for args in [&[][..], &["A", "B"][..], &["--verbose"][..], &["-x"][..]] {
        let run = sandbox.run(&sandbox.repo(), args);

        assert_eq!(
            (
                run.status.code(),
                String::from_utf8_lossy(&run.stdout).as_ref()
            ),
            (Some(2), ""),
            "{args:?}"
        );
        assert!(
            stderr(&run).starts_with("Usage: fnox-add-secret <NAME>\n"),
            "{args:?}: {}",
            stderr(&run)
        );
    }
    assert_eq!(
        (
            sandbox.file("fnox.local.toml"),
            sandbox.file("CLAUDE.local.md"),
            sandbox.fnox_calls()
        ),
        (None, None, None)
    );
}

// `--help` and `-h` print how the command is called rather than being taken for a key's
// name: nothing is written into the repository and fnox is not run.
#[test]
fn prints_its_usage_on_help_and_touches_nothing() {
    let sandbox = Fixture::new("help");
    for flag in ["--help", "-h"] {
        let run = sandbox.run(&sandbox.repo(), &[flag]);

        let stdout = String::from_utf8_lossy(&run.stdout);
        assert_eq!(
            (run.status.code(), stderr(&run)),
            (Some(0), String::new()),
            "{flag}"
        );
        assert!(
            stdout.starts_with("Usage: fnox-add-secret <NAME>\n"),
            "{flag}: {stdout}"
        );
    }
    assert_eq!(
        (
            sandbox.file("fnox.local.toml"),
            sandbox.file("CLAUDE.local.md"),
            sandbox.fnox_calls()
        ),
        (None, None, None)
    );
}

// In a repository with no fnox.local.toml, the command creates one declaring a keychain
// provider named after the repository, and has fnox store the key there.
#[test]
fn declares_the_repositorys_provider_and_stores_the_key_with_it() {
    let sandbox = Fixture::new("fresh");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.file("fnox.local.toml").as_deref(),
        Some("[providers.\"repo\"]\ntype = \"keychain\"\nservice = \"repo\"\n\n")
    );
    assert_eq!(
        sandbox.set_call(),
        Some(format!(
            "{}\nset\n-c\nfnox.local.toml\nAPI_KEY\n--provider\nrepo\n",
            sandbox.repo().display()
        ))
    );
}

// Run from a worktree, the command acts on the main checkout, whose fnox.local.toml fnox
// finds from the worktree under it, and leaves the worktree without one.
#[test]
fn acts_on_the_main_checkout_from_a_worktree() {
    let sandbox = Fixture::new("worktree");
    sandbox.git(&["worktree", "add", "--quiet", "../linked"]);
    let linked = sandbox.root().join("linked");

    let run = sandbox.run(&linked, &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        (
            sandbox.file("fnox.local.toml").is_some(),
            linked.join("fnox.local.toml").exists(),
        ),
        (true, false)
    );
    assert!(
        sandbox
            .set_call()
            .is_some_and(|call| call.starts_with(&format!("{}\n", sandbox.repo().display()))),
        "{:?}",
        sandbox.set_call()
    );
}

// Where fnox already lists the repository's provider, in any TOML form, the config is
// left as it is and the key is stored with that provider.
#[test]
fn leaves_a_config_that_declares_the_provider_as_it_is() {
    let sandbox = Fixture::new("declared");
    let config = "[providers.\"repo\"]\ntype = \"keychain\"\nservice = \"elsewhere\"\n";
    sandbox.write("fnox.local.toml", config);

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(sandbox.file("fnox.local.toml").as_deref(), Some(config));
    assert!(sandbox.set_call().is_some());
}

// A declaration appended to a config whose last line lacks a newline starts on a line of
// its own rather than being glued to that line.
#[test]
fn starts_the_declaration_on_a_line_of_its_own() {
    let sandbox = Fixture::new("no-final-newline");
    sandbox.write("fnox.local.toml", "[secrets]\nOTHER = { provider = \"x\" }");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.file("fnox.local.toml").as_deref(),
        Some(
            "[secrets]\nOTHER = { provider = \"x\" }\n\
             [providers.\"repo\"]\ntype = \"keychain\"\nservice = \"repo\"\n\n"
        )
    );
}

// A config that cannot take an appended table, such as one defining its providers as an
// inline table, is left unchanged, the key is not stored, and the user is told to declare
// the provider by hand.
#[test]
fn leaves_a_config_that_cannot_take_the_declaration_unchanged() {
    let sandbox = Fixture::new("inline-providers");
    let config = "providers = { other = { type = \"keychain\", service = \"other\" } }\n";
    sandbox.write("fnox.local.toml", config);

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(
        (run.status.code(), stderr(&run)),
        (
            Some(1),
            "fnox: cannot parse fnox.local.toml.new\n\
             fnox.local.toml cannot take the declaration as appended; left unchanged, so add a keychain provider named repo by hand\n"
                .to_owned()
        )
    );
    assert_eq!(sandbox.file("fnox.local.toml").as_deref(), Some(config));
    assert_eq!(
        (sandbox.file("fnox.local.toml.new"), sandbox.set_call()),
        (None, None)
    );
}

/// The line `CLAUDE.local.md` gets, telling an agent how secrets reach a command.
const NOTE: &str = "- Secrets here come from the macOS keychain through fnox (`fnox.local.toml`), not `.env`: run a command that needs one as `fnox exec -- <command>`.\n";

// A repository without CLAUDE.local.md gets one holding the line on how secrets reach a
// command.
#[test]
fn writes_the_note_into_a_new_claude_local_md() {
    let sandbox = Fixture::new("note-new");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!((run.status.code(), stderr(&run)), (Some(0), String::new()));
    assert_eq!(sandbox.file("CLAUDE.local.md").as_deref(), Some(NOTE));
}

// An existing CLAUDE.local.md has a structure of its own, so the command leaves it as it
// is and tells the user to add the line where it fits, unless it already says how to run
// a command under fnox.
#[test]
fn leaves_an_existing_claude_local_md_to_the_user() {
    for (test, contents, told) in [
        ("note-missing", "# Local\n", true),
        (
            "note-present",
            "Run tests as `fnox exec -- cargo test`.\n",
            false,
        ),
    ] {
        let sandbox = Fixture::new(test);
        sandbox.write("CLAUDE.local.md", contents);

        let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

        let expected = if told {
            format!(
                "Add this line where it fits in {}/CLAUDE.local.md:\n{NOTE}",
                sandbox.repo().display()
            )
        } else {
            String::new()
        };
        assert_eq!(
            (run.status.code(), stderr(&run)),
            (Some(0), expected),
            "{test}"
        );
        assert_eq!(
            sandbox.file("CLAUDE.local.md").as_deref(),
            Some(contents),
            "{test}"
        );
    }
}

// Where fnox fails to store the key, the command ends with fnox's exit code and writes
// no note, the key not being there to run a command with.
#[test]
fn ends_with_fnoxs_code_where_it_fails_to_store_the_key() {
    let sandbox = Fixture::new("set-fails");
    sandbox.fail_set(3);

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(3), "{}", stderr(&run));
    assert_eq!(sandbox.file("CLAUDE.local.md"), None);
}

// A config fnox cannot parse stops the command with fnox's error and exit code, before
// anything is appended or stored.
#[test]
fn stops_on_a_config_fnox_cannot_parse() {
    let sandbox = Fixture::new("unparseable");
    sandbox.write("fnox.local.toml", "unparseable\n");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(
        (run.status.code(), stderr(&run)),
        (Some(1), "fnox: cannot parse fnox.local.toml\n".to_owned())
    );
    assert_eq!(
        (
            sandbox.file("fnox.local.toml").as_deref(),
            sandbox.set_call()
        ),
        (Some("unparseable\n"), None)
    );
}

// The key's value reaches fnox on the command's stdin, as fnox prompts for it, and the
// command itself prints it nowhere.
#[test]
fn passes_the_value_to_fnox_without_printing_it() {
    let sandbox = Fixture::new("value");

    let run = sandbox.run_with_input(&["API_KEY"], "s3cret-value\n");

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(sandbox.set_stdin().as_deref(), Some("s3cret-value\n"));
    let printed = format!("{}{}", String::from_utf8_lossy(&run.stdout), stderr(&run));
    assert!(!printed.contains("s3cret"), "{printed}");
}

// Outside a git repository there is no checkout to act on, so the command ends with
// git's error and exit code, having touched nothing.
#[test]
fn fails_with_gits_error_outside_a_repository() {
    let sandbox = Fixture::new("outside");
    let outside = sandbox.root().join("home");

    let run = sandbox.run(&outside, &["API_KEY"]);

    assert_eq!(run.status.code(), Some(128), "{}", stderr(&run));
    assert!(
        stderr(&run).starts_with("fatal: not a git repository"),
        "{}",
        stderr(&run)
    );
    assert_eq!(
        (
            fs::read_dir(&outside).expect("home is read").count(),
            sandbox.set_call()
        ),
        (0, None)
    );
}

// Where fnox.local.toml has mode 0600 and the declaration has to be added, the file keeps
// that mode, as it did when the script copied it with `cp`.
#[test]
fn keeps_the_mode_of_a_config_it_declares_the_provider_in() {
    let sandbox = Fixture::new("keeps-mode");
    sandbox.write("fnox.local.toml", "[secrets]\n");
    let path = sandbox.repo().join("fnox.local.toml");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("the mode is set");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    let mode = fs::metadata(&path)
        .expect("the config exists")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "mode is {:o}", mode & 0o777);
}

// Where CLAUDE.local.md is a symlink whose target does not exist yet, the command writes
// the line into the target and ends cleanly, as the script's `-e` test, which follows the
// link, and its redirection did.
#[test]
fn writes_the_note_through_a_dangling_claude_local_md_symlink() {
    let sandbox = Fixture::new("note-dangling-symlink");
    std::os::unix::fs::symlink("notes.md", sandbox.repo().join("CLAUDE.local.md"))
        .expect("the symlink is created");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!((run.status.code(), stderr(&run)), (Some(0), String::new()));
    assert_eq!(sandbox.file("notes.md").as_deref(), Some(NOTE));
}

// A read-only fnox.local.toml that already declares the provider does not stop the
// command: it neither needs to write the file nor fails on the attempt, and fnox stores
// the key.
#[test]
fn stores_the_key_with_a_read_only_config_that_declares_the_provider() {
    let sandbox = Fixture::new("read-only-config");
    sandbox.write(
        "fnox.local.toml",
        "[providers.\"repo\"]\ntype = \"keychain\"\nservice = \"repo\"\n",
    );
    let path = sandbox.repo().join("fnox.local.toml");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).expect("the mode is set");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert!(sandbox.set_call().is_some(), "fnox set was not called");
}

// Only a provider named exactly after the repository counts as declared: with other
// providers, one merely containing the repository's name among them, the command still
// appends the repository's declaration and stores the key with that provider, as the
// script's `grep -qxF` matched whole lines only.
#[test]
fn declares_the_provider_where_only_others_are_listed() {
    let sandbox = Fixture::new("others-only");
    let config = "[providers.\"other\"]\ntype = \"keychain\"\nservice = \"other\"\n\n\
                  [providers.\"repo-x\"]\ntype = \"keychain\"\nservice = \"repo-x\"\n";
    sandbox.write("fnox.local.toml", config);

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        sandbox.file("fnox.local.toml"),
        Some(format!(
            "{config}[providers.\"repo\"]\ntype = \"keychain\"\nservice = \"repo\"\n\n"
        ))
    );
    assert_eq!(
        sandbox.set_call(),
        Some(format!(
            "{}\nset\n-c\nfnox.local.toml\nAPI_KEY\n--provider\nrepo\n",
            sandbox.repo().display()
        ))
    );
}

// Where fnox.local.toml is a symlink whose target does not exist yet, the command
// creates the target, as the script's `touch` did, and goes on to declare the provider
// and store the key; the declared config replaces the link, as the script's `mv` did,
// leaving the target empty.
#[test]
fn creates_the_target_of_a_dangling_config_symlink() {
    let sandbox = Fixture::new("config-dangling-symlink");
    std::os::unix::fs::symlink("shared.toml", sandbox.repo().join("fnox.local.toml"))
        .expect("the symlink is created");

    let run = sandbox.run(&sandbox.repo(), &["API_KEY"]);

    assert_eq!(run.status.code(), Some(0), "{}", stderr(&run));
    assert_eq!(
        (
            sandbox.file("shared.toml").as_deref(),
            sandbox.file("fnox.local.toml").as_deref()
        ),
        (
            Some(""),
            Some("[providers.\"repo\"]\ntype = \"keychain\"\nservice = \"repo\"\n\n")
        )
    );
    assert!(sandbox.set_call().is_some(), "fnox set was not called");
}
