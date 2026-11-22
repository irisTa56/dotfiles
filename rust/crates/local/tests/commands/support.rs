//! What the command tests share: a directory of each test's own, fakes written into its
//! `bin`, and runs of the built binary, with git kept to the test's own repositories.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock, PoisonError};

/// The built `dotfiles-local`, the binary every test runs.
pub(crate) const DOTFILES_LOCAL: &str = env!("CARGO_BIN_EXE_dotfiles-local");

/// The author and committer a test's git commits are made with, since the machine may
/// have none, set as environment variables.
pub(crate) const IDENTITY: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "Test"),
    ("GIT_AUTHOR_EMAIL", "test@example.com"),
    ("GIT_COMMITTER_NAME", "Test"),
    ("GIT_COMMITTER_EMAIL", "test@example.com"),
];

/// A directory of one test's own, holding a `bin` for the fakes it puts on PATH.
pub(crate) struct Sandbox {
    root: PathBuf,
    lifetime: Lifetime,
}

/// What becomes of a sandbox once the test is done with it.
enum Lifetime {
    /// It stays to be looked at, until the test's next run replaces it.
    Kept,
    /// It is removed, since nothing else clears the directory it lies in.
    Removed,
}

impl Sandbox {
    /// A fresh, empty sandbox for `test` among the tests of `command`, under this
    /// build's `target/`, with `bin` and each of `dirs` created under it. What an earlier
    /// run left there is removed first, and what this run leaves stays to be looked at.
    pub(crate) fn new(command: &str, test: &str, dirs: &[&str]) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(command)
            .join(test);
        Self::create(root, Lifetime::Kept, dirs)
    }

    /// A fresh, empty sandbox for `test` among the tests of `command`, under the system's
    /// temporary directory and so outside this repository, with `bin` and each of `dirs`
    /// created under it. It is removed when dropped.
    pub(crate) fn outside_repository(command: &str, test: &str, dirs: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "dotfiles-local-{command}-{}-{test}",
            std::process::id()
        ));
        Self::create(root, Lifetime::Removed, dirs)
    }

    fn create(root: PathBuf, lifetime: Lifetime, dirs: &[&str]) -> Self {
        let _ = fs::remove_dir_all(&root);
        for dir in std::iter::once("bin").chain(dirs.iter().copied()) {
            fs::create_dir_all(root.join(dir)).expect("the sandbox is created");
        }
        Sandbox { root, lifetime }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// The directory the fakes are written into.
    pub(crate) fn bin(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// A PATH that finds the fakes before what the tests' own PATH holds.
    pub(crate) fn path_before_inherited(&self) -> OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        std::env::join_paths(std::iter::once(self.bin()).chain(std::env::split_paths(&inherited)))
            .expect("PATH is joined")
    }

    /// Writes a fake `name` into `bin` that runs `body` as a `/bin/sh` script.
    pub(crate) fn fake(&self, name: &str, body: &str) {
        self.fake_run_by(name, "/bin/sh", body);
    }

    /// Writes into `bin` a fake of each of `programs` that fails saying it was run, so a
    /// run with `bin` first on PATH shows the command does without them even on a
    /// machine that has them.
    pub(crate) fn forbid(&self, programs: &[&str]) {
        for program in programs {
            self.fake(program, &format!("echo '{program} was run' >&2\nexit 99"));
        }
    }

    /// Writes a fake `name` into `bin` that runs `body` as a script of `interpreter`, a
    /// shebang line's command.
    ///
    /// The fake is a symlink to a launcher shared by every fake of `interpreter`, which
    /// sources `body` from `<name>.body` beside the link, so `$0` and the arguments are
    /// the fake's own. macOS assesses an executable the first time it runs, taking about
    /// 0.2 s for content it has not seen before, and a body naming the sandbox would be
    /// such content on each run; the launcher's content is always the same.
    pub(crate) fn fake_run_by(&self, name: &str, interpreter: &str, body: &str) {
        let path = self.bin().join(name);
        fs::write(body_of(&path), format!("{body}\n")).expect("the fake's body is written");
        let _ = fs::remove_file(&path);
        symlink(launcher(interpreter), &path).expect("the fake is linked to its launcher");
    }

    /// Renames the fake `from` in `bin` to `to`, its body with it.
    pub(crate) fn rename_fake(&self, from: &str, to: &str) {
        let (from, to) = (self.bin().join(from), self.bin().join(to));
        fs::rename(&from, &to).expect("the fake is renamed");
        fs::rename(body_of(&from), body_of(&to)).expect("the fake's body is renamed");
    }
}

/// Where the launcher of the fake at `path` finds its body.
fn body_of(path: &Path) -> PathBuf {
    let mut body = path.as_os_str().to_owned();
    body.push(".body");
    PathBuf::from(body)
}

/// The launcher the fakes run by `interpreter` link to, written under this build's
/// `target/` the first time a test asks for it. It is written aside and renamed into
/// place, so a test running in another process at the same time sees it whole or not
/// at all.
fn launcher(interpreter: &str) -> PathBuf {
    static WRITING: Mutex<()> = Mutex::new(());
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fake-launchers");
    let name: String = interpreter
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let path = dir.join(&name);
    let _writing = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    if !path.exists() {
        fs::create_dir_all(&dir).expect("the launchers' directory is created");
        let aside = dir.join(format!("{name}.{}", std::process::id()));
        fs::write(&aside, format!("#!{interpreter}\n. \"$0.body\"\n"))
            .expect("the launcher is written");
        fs::set_permissions(&aside, fs::Permissions::from_mode(0o755))
            .expect("the launcher is made executable");
        fs::rename(&aside, &path).expect("the launcher is put in place");
    }
    path
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Lifetime::Removed = self.lifetime {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// The git the tests' own setup and reads run: the program in git's exec path, which the
/// `git` on PATH may only reach through a shim, such as the one macOS puts in `/usr/bin`
/// that looks up the developer tools on every run. Where the exec path holds no `git`,
/// the one on PATH. The binary under test runs the `git` on the PATH a test gives it.
pub(crate) fn git() -> &'static Path {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(|| {
        let output = Command::new("git")
            .arg("--exec-path")
            .output()
            .expect("git runs");
        let exec_path = String::from_utf8(output.stdout).expect("git's exec path is UTF-8");
        let git = Path::new(exec_path.trim_end()).join("git");
        if output.status.success() && git.is_file() {
            git
        } else {
            PathBuf::from("git")
        }
    })
}

/// `dotfiles-local` run as `subcommand`, with the environment the tests run in.
pub(crate) fn dotfiles_local(subcommand: &str) -> Command {
    let mut command = Command::new(DOTFILES_LOCAL);
    command.arg(subcommand);
    command
}

/// `program` run in `dir` with git, run by it or by the binary it runs, kept to the
/// repositories under `ceiling` and away from the machine's config: a git hook running
/// the tests exports `GIT_DIR` and its kin for its own repository, and a directory above
/// the sandbox is one.
pub(crate) fn isolated(program: impl AsRef<OsStr>, dir: &Path, ceiling: &Path) -> Command {
    let mut command = Command::new(program);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CEILING_DIRECTORIES", ceiling);
    command
}

/// Runs `command` with `input` as its stdin and what it writes captured. The command
/// need not read its input, so a write the closed pipe refuses is not a failure.
pub(crate) fn output_with_input(mut command: Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the command starts");
    let _ = child.stdin.take().expect("stdin is piped").write_all(input);
    child.wait_with_output().expect("the command ends")
}
