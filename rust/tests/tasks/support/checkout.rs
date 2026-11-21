//! The repository a run of the binary, its wrapper or a shell task checks, and the runs
//! that can be made in it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use super::git::{IDENTITY, git, isolated};
use super::invocation::Invocation;
use super::mise::Mise;
use super::origin::Origin;

/// The repository a run checks: a fresh git repository with the files a test stages.
pub(crate) struct Checkout {
    dir: TempDir,
}

impl Checkout {
    /// A repository holding `files`, given as path and content, staged.
    pub(crate) fn new(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let checkout = Checkout { dir };
        checkout.git(&["init", "-q"]);
        for (path, content) in files {
            checkout.write(path, content);
        }
        checkout.git(&["add", "-A"]);
        checkout
    }

    /// A repository holding `contents` in the one file `path`, staged; unlike `new`'s,
    /// the contents may be any bytes.
    pub(crate) fn holding(path: &str, contents: &[u8]) -> Self {
        let checkout = Checkout::new(&[]);
        checkout.write(path, contents);
        checkout.git(&["add", "-A"]);
        checkout
    }

    /// A clone of `origin`'s `main` holding its tip alone, as `actions/checkout` leaves
    /// one by default.
    pub(crate) fn shallow_clone(origin: &Origin) -> Self {
        let dir = TempDir::new().expect("a temporary directory can be made");
        let checkout = Checkout { dir };
        // Only a URL, not a local path, makes git honour the depth.
        let url = format!("file://{}", origin.location());
        checkout.git(&["clone", "-q", "--depth=1", &url, "."]);
        checkout
    }

    /// Writes `contents` to `path` in the working tree, staging nothing.
    pub(crate) fn write(&self, path: &str, contents: impl AsRef<[u8]>) {
        let file = self.dir.path().join(path);
        fs::create_dir_all(file.parent().expect("a file has a parent"))
            .expect("the file's directory can be made");
        fs::write(&file, contents).expect("the file can be written");
    }

    /// Removes `path`, a file or a directory, from the working tree, staging nothing;
    /// removing `.git` leaves a directory git does not track.
    pub(crate) fn remove(&self, path: &str) {
        let target = self.dir.path().join(path);
        if target.is_dir() {
            fs::remove_dir_all(target).expect("the directory can be removed");
        } else {
            fs::remove_file(target).expect("the file can be removed");
        }
    }

    /// Makes the directory `path` in the working tree, staging nothing.
    pub(crate) fn make_dir(&self, path: &str) {
        fs::create_dir_all(self.dir.path().join(path)).expect("the directory can be made");
    }

    /// Takes every permission off `path` in the working tree, so that it cannot be read.
    pub(crate) fn lock(&self, path: &str) {
        fs::set_permissions(
            self.dir.path().join(path),
            fs::Permissions::from_mode(0o000),
        )
        .expect("the file's mode can be set");
    }

    /// Gives `path` in the working tree the mode a directory is made with, undoing `lock`
    /// so that the temporary directory can be removed.
    pub(crate) fn unlock(&self, path: &str) {
        fs::set_permissions(
            self.dir.path().join(path),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the directory's mode can be set");
    }

    /// Whether the working tree holds anything at `path`.
    pub(crate) fn exists(&self, path: &str) -> bool {
        self.dir.path().join(path).exists()
    }

    /// The bytes of `path` in the working tree.
    pub(crate) fn read(&self, path: &str) -> Vec<u8> {
        fs::read(self.dir.path().join(path)).expect("the file can be read")
    }

    /// Runs git in the repository, as setup that must succeed, and returns its trimmed
    /// stdout.
    pub(crate) fn git(&self, args: &[&str]) -> String {
        git(self.dir.path(), args)
    }

    /// Commits what is staged and makes it `origin`'s `main`.
    pub(crate) fn publish(&self, origin: &Origin) {
        self.git(&[&IDENTITY[..], &["commit", "-qm", "base"]].concat());
        self.git(&["branch", "-M", "main"]);
        self.git(&["remote", "add", "origin", origin.location()]);
        self.git(&["push", "-q", "origin", "main"]);
    }

    /// Names `origin` as the repository's remote `name`, fetching nothing.
    pub(crate) fn add_remote(&self, name: &str, origin: &Origin) {
        self.git(&["remote", "add", name, origin.location()]);
    }

    /// Puts the repository on `origin`'s `main` with its own files staged over it, so
    /// that the checkout differs from origin.
    pub(crate) fn stage_over(&self, origin: &Origin) {
        self.git(&["remote", "add", "origin", origin.location()]);
        self.git(&["fetch", "-q", "origin", "main"]);
        self.git(&["reset", "-q", "origin/main"]);
        self.git(&["add", "-A"]);
    }

    /// A run of the built binary in the repository, naming no subcommand until one is
    /// passed as its first argument.
    pub(crate) fn dotfiles_tasks(&self) -> Invocation {
        self.invocation(isolated(
            env!("CARGO_BIN_EXE_dotfiles-tasks"),
            self.dir.path(),
        ))
    }

    /// A run of the wrapper consumers run at `path` under `tasks/`, in the repository,
    /// finding `mise` in place of mise.
    pub(crate) fn wrapper(&self, path: &str, mise: &Mise) -> Invocation {
        let wrapper = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tasks")
            .join(path);
        let mut command = isolated("bash", self.dir.path());
        // A run under a caller that skips mise's hooks would pass whether or not the
        // wrapper skips them itself.
        command
            .arg(wrapper)
            .env("PATH", mise.path())
            .env_remove("MISE_NO_HOOKS");
        self.invocation(command)
    }

    /// A run of the shell task consumers run at `path` under `tasks/`, in the repository,
    /// finding what it runs on the PATH the tests run with.
    pub(crate) fn shell_task(&self, path: &str) -> Invocation {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tasks")
            .join(path);
        let mut command = isolated("bash", self.dir.path());
        command.arg(script);
        self.invocation(command)
    }

    fn invocation(&self, command: Command) -> Invocation {
        Invocation::new(command, self.dir.path().to_path_buf())
    }
}
