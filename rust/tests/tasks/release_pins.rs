//! That each shared task wrapping the `dotfiles-tasks` binary pins the release this
//! package is at. The release pull request moves both together, and release-please
//! skips a file its config names but cannot find without saying so, so a wrapper moved
//! or renamed would go on pinning an old release unnoticed.

use std::fs;
use std::path::Path;

use regex::Regex;
use tempfile::TempDir;

/// What release-please looks for on the line that holds the version it moves.
const MARKER: &str = "x-release-please-version";

/// The shared tasks under `tasks/` in the repository at `root`.
struct Tasks<'a> {
    root: &'a Path,
}

impl Tasks<'_> {
    /// What is amiss against the package at `version`, one line each; empty where
    /// nothing is.
    fn mismatches(&self, version: &str) -> Vec<String> {
        let pinned =
            Regex::new(&format!(r#"@([0-9][^'"\s]*).*{MARKER}"#)).expect("the pattern is valid");
        let wrappers = self.wrappers();
        if wrappers.is_empty() {
            return vec![format!(
                "no file under tasks/ carries {MARKER}, so no wrapper pins a release"
            )];
        }
        wrappers
            .into_iter()
            .filter_map(|(path, text)| match pinned.captures(&text) {
                None => Some(format!(
                    "{path} carries {MARKER} on a line with no @<version>"
                )),
                Some(found) if &found[1] == version => None,
                Some(found) => Some(format!(
                    "{path} pins {}, but rust/Cargo.toml is at {version}",
                    &found[1]
                )),
            })
            .collect()
    }

    /// Each shell task that carries the marker, as its path from `root` and its text,
    /// in path order.
    fn wrappers(&self) -> Vec<(String, String)> {
        let mut wrappers = Vec::new();
        let mut dirs = vec![self.root.join("tasks")];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).expect("the directory can be read") {
                let path = entry.expect("the entry can be read").path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.is_file() && path.extension().is_some_and(|e| e == "sh") {
                    let text = fs::read_to_string(&path).expect("the task can be read");
                    if text.contains(MARKER) {
                        let relative = path
                            .strip_prefix(self.root)
                            .expect("the task is under root");
                        wrappers.push((relative.display().to_string(), text));
                    }
                }
            }
        }
        wrappers.sort();
        wrappers
    }
}

/// Writes `content` to `path` under `root`, making its directories.
fn put(root: &Path, path: &str, content: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a file has a parent"))
        .expect("the file's directory can be made");
    fs::write(file, content).expect("the file can be written");
}

/// Writes a wrapper at `path` under `root` pinning `version`, marked as the shared tasks'
/// wrappers are.
fn wrapper(root: &Path, path: &str, version: &str) {
    put(
        root,
        path,
        &format!(
            "#!/usr/bin/env bash\nexec mise x 'github:o/r[version_prefix=x-v,bin=x]@{version}' -- x \"$@\" # x-release-please-version\n"
        ),
    );
}

#[test]
fn names_a_wrapper_that_pins_another_version_than_the_package() {
    let root = TempDir::new().expect("a temporary directory can be made");
    wrapper(root.path(), "tasks/a/b.sh", "0.1.0");

    assert_eq!(
        Tasks { root: root.path() }.mismatches("0.2.0"),
        ["tasks/a/b.sh pins 0.1.0, but rust/Cargo.toml is at 0.2.0"]
    );
}

// A version line that lost its marker, or a wrapper gone from tasks/, would leave nothing
// to compare, and the check would pass having checked nothing.
#[test]
fn reports_where_no_shared_task_carries_the_marker() {
    let root = TempDir::new().expect("a temporary directory can be made");
    put(
        root.path(),
        "tasks/a/b.sh",
        "#!/usr/bin/env bash\necho hi\n",
    );

    assert_eq!(
        Tasks { root: root.path() }.mismatches("0.2.0"),
        ["no file under tasks/ carries x-release-please-version, so no wrapper pins a release"]
    );
}

// A marked line without a version, such as `@latest`, is amiss like any other; the check
// must say so rather than pass it over.
#[test]
fn names_a_wrapper_whose_marked_line_has_no_version() {
    let root = TempDir::new().expect("a temporary directory can be made");
    put(
        root.path(),
        "tasks/a/b.sh",
        "#!/usr/bin/env bash\nexec mise x 'github:o/r@latest' -- x \"$@\" # x-release-please-version\n",
    );

    assert_eq!(
        Tasks { root: root.path() }.mismatches("0.2.0"),
        ["tasks/a/b.sh carries x-release-please-version on a line with no @<version>"]
    );
}

// tasks/README.md names the marker in its prose; a task is a `.sh` file.
#[test]
fn reads_the_marker_only_from_shell_tasks() {
    let root = TempDir::new().expect("a temporary directory can be made");
    wrapper(root.path(), "tasks/a/b.sh", "0.2.0");
    put(
        root.path(),
        "tasks/README.md",
        "Its version line ends in `# x-release-please-version @0.1.0`.\n",
    );

    assert_eq!(
        Tasks { root: root.path() }.mismatches("0.2.0"),
        Vec::<String>::new()
    );
}

#[test]
fn every_wrapper_in_this_repository_pins_the_release_this_package_is_at() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mismatches = Tasks { root: &root }.mismatches(env!("CARGO_PKG_VERSION"));
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
