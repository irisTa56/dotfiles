//! `upstream:pin-check`: whether the upstream paths this repository pins have changed on
//! their default branches, and with `--bump`, moving the pins that have.
//!
//! What a pin is, and what each exit code means, is in tasks/README.md under
//! "upstream:pin-check".

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::{env, fs, io};

use regex::{Captures, Regex};
use tempfile::TempDir;

use crate::git::{Git, Run};

/// How a run ends, ordered so that the worst of several outcomes is their maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Status {
    /// Nothing to act on.
    Ok = 0,
    /// A pinned path differs from its upstream's default branch.
    Differs = 1,
    /// It could not tell.
    Unknown = 2,
}

/// Once this branch exists it is the person's who opens its pull request, who may add
/// commits to it, so a later run leaves it alone; merging that pull request deletes it where
/// the repository deletes merged branches, and the next run pushes afresh.
const BUMP_BRANCH: &str = "upstream-pin-bump";
const BOT_NAME: &str = "github-actions[bot]";
const BOT_EMAIL: &str = "41898282+github-actions[bot]@users.noreply.github.com";

/// A marker is the form mise's `git::` include takes, so a `task_config.includes` entry is
/// one as it stands. The URL is a scheme, a host and the path segments after it, ending at
/// the first `//` past the scheme's, which is where mise splits it too; requiring the scheme
/// keeps a URL from opening with `-`, which git would take as an option. Only a full sha
/// counts: a branch or a short sha moves, so there would be no fixed commit to compare
/// against, and [`markers`] drops a match that more hex digits follow.
static MARKER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"git::([a-z][a-z0-9+.-]*://[^\s"'?/]*(?:/[^\s"'?/]+)*?)//([^\s"'?]+)\?ref=([0-9a-f]{40})"#,
    )
    .expect("MARKER is a valid pattern")
});

static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(https://github\.com/[^/]+/[^/]+?)(?:\.git)?$")
        .expect("GITHUB is a valid pattern")
});

/// One pin: an upstream URL, a path in it, and the commit pinned, with the tracked files
/// that spell it.
#[derive(Debug)]
struct Pin {
    url: String,
    path: String,
    commit: String,
    files: Vec<String>,
}

/// A pin that differs, and the default branch's tip it would move to.
#[derive(Debug)]
struct Move<'p> {
    pin: &'p Pin,
    tip: String,
}

/// A pin as one text spelled it, and the tip the move put there in its place.
#[derive(Debug)]
struct Moved {
    url: String,
    path: String,
    from: String,
    tip: String,
}

/// What checking one pin found.
#[derive(Debug)]
enum Checked {
    Matches,
    /// The path is gone from the default branch, so there is nothing to move the pin to.
    Gone,
    /// The path differs from the one at this default branch tip.
    Differs(String),
    Unknown,
}

impl Checked {
    fn status(&self) -> Status {
        match self {
            Checked::Matches => Status::Ok,
            Checked::Gone | Checked::Differs(_) => Status::Differs,
            Checked::Unknown => Status::Unknown,
        }
    }
}

fn fail(message: &str) {
    eprintln!("[fail] {message}");
}

/// git's own reason goes out above the line that says what could not be done.
fn fail_git(run: &Run, message: &str) -> Status {
    eprint!("{}", run.stderr);
    fail(message);
    Status::Unknown
}

fn marker(url: &str, path: &str, commit: &str) -> String {
    format!("git::{url}//{path}?ref={commit}")
}

/// The markers in `text`, as their captures: the URL, the path and the commit.
fn markers(text: &str) -> impl Iterator<Item = Captures<'_>> {
    MARKER.captures_iter(text).filter(|found| {
        let end = found.get(0).expect("a match has a whole").end();
        !text[end..]
            .bytes()
            .next()
            .is_some_and(|next| next.is_ascii_digit() || (b'a'..=b'f').contains(&next))
    })
}

/// Moves each marker in `text` that `tip_for` gives a tip for, and returns the moved text
/// with what moved, each move read from the marker it replaced.
fn rewrite(
    text: &str,
    mut tip_for: impl FnMut(&str, &str, &str) -> Option<String>,
) -> (String, Vec<Moved>) {
    let mut moved = Vec::new();
    let mut result = String::with_capacity(text.len());
    let mut last = 0;
    for found in markers(text) {
        let (url, path, from) = (&found[1], &found[2], &found[3]);
        let Some(tip) = tip_for(url, path, from).filter(|tip| tip != from) else {
            continue;
        };
        let whole = found.get(0).expect("a match has a whole");
        result.push_str(&text[last..whole.start()]);
        result.push_str(&marker(url, path, &tip));
        last = whole.end();
        moved.push(Moved {
            url: url.to_owned(),
            path: path.to_owned(),
            from: from.to_owned(),
            tip,
        });
    }
    result.push_str(&text[last..]);
    (result, moved)
}

/// What the move brings, as a page where the host has one.
fn brings(url: &str, from: &str, to: &str) -> String {
    match GITHUB.captures(url) {
        Some(github) => format!("{}/compare/{from}...{to}", &github[1]),
        None => format!("{url} {from}...{to}"),
    }
}

/// One line per distinct move, however many files or markers it touched.
fn describe(moved: &[Moved]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for Moved {
        url,
        path,
        from,
        tip,
    } in moved
    {
        let line = format!(
            "{url}//{path} to {tip}; what it brings: {}",
            brings(url, from, tip)
        );
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    lines
}

fn read_pins(files: &[&str]) -> io::Result<Vec<Pin>> {
    let mut pins: Vec<Pin> = Vec::new();
    let mut seen: HashMap<(String, String, String), usize> = HashMap::new();
    for &file in files {
        let text = String::from_utf8_lossy(&fs::read(file)?).into_owned();
        for found in markers(&text) {
            let key = (
                found[1].to_owned(),
                found[2].to_owned(),
                found[3].to_owned(),
            );
            let index = *seen.entry(key).or_insert_with_key(|(url, path, commit)| {
                pins.push(Pin {
                    url: url.clone(),
                    path: path.clone(),
                    commit: commit.clone(),
                    files: Vec::new(),
                });
                pins.len() - 1
            });
            let pin = &mut pins[index];
            if !pin.files.iter().any(|known| known == file) {
                pin.files.push(file.to_owned());
            }
        }
    }
    Ok(pins)
}

/// Runs the check on `args`, `--bump` and then git pathspecs, and returns how it ended.
pub(crate) fn run(args: &[String]) -> Status {
    match check_pins(args) {
        Ok(status) => status,
        // An error from the filesystem is no answer about the pins, so it exits as one.
        Err(error) => {
            fail(&format!("stopped on {error}"));
            Status::Unknown
        }
    }
}

fn check_pins(args: &[String]) -> io::Result<Status> {
    let bump = args.first().is_some_and(|first| first == "--bump");
    let pathspecs = if bump { &args[1..] } else { args };
    if let Some(option) = pathspecs.iter().find(|arg| arg.starts_with('-')) {
        fail(&format!(
            "unknown argument '{option}'; this takes --bump, then git pathspecs"
        ));
        return Ok(Status::Unknown);
    }
    let git = Git::new();
    // Names from the repository's root, which is where the rest of the run works, so that a
    // file reads the same from the working tree and from the default branch's tree.
    let mut grep = vec!["grep", "-lzI", "--full-name", "-e", "git::", "--"];
    grep.extend(pathspecs.iter().map(String::as_str));
    let listed = git.run(&grep);
    if listed.status > 1 {
        return Ok(fail_git(
            &listed,
            "could not list the tracked files to read pins from",
        ));
    }
    let root = git.run(&["rev-parse", "--show-toplevel"]);
    if !root.success() {
        return Ok(fail_git(&root, "could not find the repository's root"));
    }
    env::set_current_dir(root.stdout.trim())?;
    let files: Vec<&str> = listed
        .stdout
        .split('\0')
        .filter(|f| !f.is_empty())
        .collect();
    let pins = read_pins(&files)?;
    if pins.is_empty() {
        fail("found no upstream pin in the tracked files");
        return Ok(Status::Unknown);
    }

    let scratch = Scratch::new(git)?;
    let init = scratch.git(&["init", "-q"]);
    if !init.success() {
        return Ok(fail_git(
            &init,
            &format!(
                "could not create a scratch repository at {}",
                scratch.dir().display()
            ),
        ));
    }
    let mut status = Status::Ok;
    let mut differing = Vec::new();
    for pin in &pins {
        let checked = scratch.check(pin);
        status = status.max(checked.status());
        if let Checked::Differs(tip) = checked {
            differing.push(Move { pin, tip });
        }
    }
    if bump && !differing.is_empty() {
        let moved = if env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true") {
            scratch.move_on_default_branch(&differing)
        } else {
            move_in_working_tree(&differing)?
        };
        status = status.max(moved);
    }
    Ok(status)
}

/// Outside GitHub Actions the move lands in the working tree, for whoever ran it to commit.
fn move_in_working_tree(differing: &[Move]) -> io::Result<Status> {
    let tips: HashMap<String, &str> = differing
        .iter()
        .map(|Move { pin, tip }| (marker(&pin.url, &pin.path, &pin.commit), tip.as_str()))
        .collect();
    let mut moved = Vec::new();
    for file in files_of(differing) {
        let text = String::from_utf8_lossy(&fs::read(file)?).into_owned();
        let (text, file_moved) = rewrite(&text, |url, path, commit| {
            tips.get(&marker(url, path, commit))
                .map(|tip| (*tip).to_owned())
        });
        fs::write(file, text)?;
        moved.extend(file_moved);
    }
    for line in describe(&moved) {
        fail(&format!("moved the pin of {line}"));
    }
    Ok(Status::Differs)
}

/// Each file that spells one of the `differing` pins, once, in the order first met.
fn files_of<'m>(differing: &'m [Move]) -> Vec<&'m str> {
    let mut files: Vec<&str> = Vec::new();
    for file in differing.iter().flat_map(|m| &m.pin.files) {
        if !files.contains(&file.as_str()) {
            files.push(file);
        }
    }
    files
}

/// A scratch repository that upstream commits are fetched into, trees without blobs, and
/// removed with it.
#[derive(Debug)]
struct Scratch {
    git: Git,
    dir: TempDir,
}

impl Scratch {
    fn new(git: Git) -> io::Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("upstream-pin-check-")
            .tempdir()?;
        Ok(Scratch { git, dir })
    }

    fn dir(&self) -> &Path {
        self.dir.path()
    }

    fn git(&self, args: &[&str]) -> Run {
        self.git.call(args).dir(self.dir()).run()
    }

    /// Fetches `commit` of `url`, and returns it as a full sha; only the trees are
    /// compared, so no file's content is downloaded.
    fn fetch(&self, url: &str, commit: &str) -> Result<String, Run> {
        let fetched = self.git(&[
            "fetch",
            "-q",
            "--depth=1",
            "--filter=blob:none",
            url,
            commit,
        ]);
        if !fetched.success() {
            return Err(fetched);
        }
        let head = self.git(&["rev-parse", "FETCH_HEAD"]);
        if head.success() {
            Ok(head.stdout.trim().to_owned())
        } else {
            Err(head)
        }
    }

    fn tree_at(&self, commit: &str, path: &str) -> Option<String> {
        let tree = self.git(&["rev-parse", "-q", "--verify", &format!("{commit}:{path}")]);
        tree.success().then(|| tree.stdout.trim().to_owned())
    }

    fn check(&self, pin: &Pin) -> Checked {
        let place = format!("{}//{}", pin.url, pin.path);
        let tip = match self.fetch(&pin.url, "HEAD") {
            Ok(tip) => tip,
            Err(run) => {
                fail_git(
                    &run,
                    &format!("could not read the default branch of {}", pin.url),
                );
                return Checked::Unknown;
            }
        };
        let tip_tree = self.tree_at(&tip, &pin.path);
        let pinned = match self.fetch(&pin.url, &pin.commit) {
            Ok(pinned) => pinned,
            Err(run) => {
                fail_git(
                    &run,
                    &format!("could not read {} from {}", pin.commit, pin.url),
                );
                return Checked::Unknown;
            }
        };
        let Some(pinned_tree) = self.tree_at(&pinned, &pin.path) else {
            fail(&format!(
                "{place} does not exist at the pinned {}",
                pin.commit
            ));
            return Checked::Unknown;
        };
        let files = pin.files.join(", ");
        // A pin moved to a tip without the path would fail every run after, so it stays.
        let Some(tip_tree) = tip_tree else {
            fail(&format!(
                "{place} is gone from the default branch ({tip}), pinned in {files}; a bump leaves this pin where it is"
            ));
            return Checked::Gone;
        };
        if pinned_tree == tip_tree {
            println!(
                "[ok] {place} at the pinned {} matches the default branch ({tip})",
                pin.commit
            );
            return Checked::Matches;
        }
        // Not which is the older: the pin may be ahead of the default branch as well as behind.
        fail(&format!(
            "{place} differs from the default branch, pinned in {files}"
        ));
        fail(&format!("pinned: {}", pin.commit));
        fail(&format!("tip:    {tip}"));
        Checked::Differs(tip)
    }

    /// In GitHub Actions the checkout is whatever the run was for, often a pull request's
    /// merge commit, so the move is made on the repository's default branch instead and
    /// pushed to a branch of its own. A person opens the pull request from it: opening one
    /// with the job's token needs a repository setting that also lets that token approve
    /// pull requests.
    fn move_on_default_branch(&self, differing: &[Move]) -> Status {
        let git = &self.git;
        let bump_ref = format!("refs/heads/{BUMP_BRANCH}");
        let exists = git.run(&["ls-remote", "-q", "--exit-code", "origin", &bump_ref]);
        if exists.success() {
            fail(&format!(
                "{BUMP_BRANCH} already exists on origin, so it is left as it is"
            ));
            return Status::Differs;
        }
        if exists.status != 2 {
            return fail_git(
                &exists,
                &format!("could not tell whether {BUMP_BRANCH} exists on origin"),
            );
        }

        let fetched = git.run(&["fetch", "-q", "--depth=1", "origin", "HEAD"]);
        if !fetched.success() {
            return fail_git(&fetched, "could not fetch origin's default branch");
        }
        let base = git
            .run(&["rev-parse", "FETCH_HEAD"])
            .stdout
            .trim()
            .to_owned();

        let (tree, moved) = match self.stage_moves(&base, differing) {
            Ok(staged) => staged,
            Err(status) => return status,
        };
        self.push_move(&base, &tree, &moved)
    }

    /// Builds the tree of the move without touching the working tree or its index: a
    /// scratch index over the default branch's tree, with each moved file swapped in at the
    /// mode it had there. The default branch may pin another commit than the checkout, or
    /// lack a pin the checkout has, so what moves, and from where, is read from its own
    /// text, and a pin there moves only where its own path differs from the tip's.
    fn stage_moves(&self, base: &str, differing: &[Move]) -> Result<(String, Vec<Moved>), Status> {
        let git = &self.git;
        let tips: HashMap<(&str, &str), &str> = differing
            .iter()
            .map(|Move { pin, tip }| ((pin.url.as_str(), pin.path.as_str()), tip.as_str()))
            .collect();
        let tip_for = |url: &str, path: &str, commit: &str| {
            let tip = *tips.get(&(url, path))?;
            if tip == commit {
                return None;
            }
            let Ok(pinned) = self.fetch(url, commit) else {
                return Some(tip.to_owned());
            };
            (self.tree_at(&pinned, path) != self.tree_at(tip, path)).then(|| tip.to_owned())
        };
        let index_file = self.dir().join("bump-index");
        let index_file = index_file.to_string_lossy();
        let indexed = |args: &[&str]| git.call(args).env("GIT_INDEX_FILE", &index_file).run();
        let read = indexed(&["read-tree", base]);
        if !read.success() {
            return Err(fail_git(&read, "could not read origin's default branch"));
        }
        let mut moved = Vec::new();
        for file in files_of(differing) {
            let listed = git.run(&["ls-tree", base, "--", file]);
            if !listed.success() {
                return Err(fail_git(
                    &listed,
                    &format!("could not read {file} on origin's default branch"),
                ));
            }
            let fields: Vec<&str> = listed.stdout.split_whitespace().collect();
            let [mode, _, _, ..] = fields[..] else {
                continue;
            };
            let before = git.run(&["cat-file", "blob", &format!("{base}:{file}")]);
            if !before.success() {
                return Err(fail_git(
                    &before,
                    &format!("could not read {file} on origin's default branch"),
                ));
            }
            let (after, file_moved) = rewrite(&before.stdout, tip_for);
            if file_moved.is_empty() {
                continue;
            }
            let blob = git
                .call(&["hash-object", "-w", "--stdin"])
                .input(&after)
                .run()
                .stdout
                .trim()
                .to_owned();
            let update = indexed(&[
                "update-index",
                "--cacheinfo",
                &format!("{mode},{blob},{file}"),
            ]);
            if !update.success() {
                return Err(fail_git(
                    &update,
                    &format!("could not stage the move of {file}"),
                ));
            }
            moved.extend(file_moved);
        }
        if moved.is_empty() {
            fail(
                "origin's default branch holds none of these pins to move, or holds them where the path matches the tip's already",
            );
            return Err(Status::Unknown);
        }
        let tree = indexed(&["write-tree"]).stdout.trim().to_owned();
        Ok((tree, moved))
    }

    /// Commits `tree` over `base` as the bot and pushes it to [`BUMP_BRANCH`].
    fn push_move(&self, base: &str, tree: &str, moved: &[Moved]) -> Status {
        let git = &self.git;
        let lines = describe(moved);
        let mut message = String::from("chore: move upstream pins to their default branches\n\n");
        for line in &lines {
            message.push_str("- ");
            message.push_str(line);
            message.push('\n');
        }
        let commit = git
            .call(&["commit-tree", tree, "-p", base, "-F", "-"])
            .env("GIT_AUTHOR_NAME", BOT_NAME)
            .env("GIT_AUTHOR_EMAIL", BOT_EMAIL)
            .env("GIT_COMMITTER_NAME", BOT_NAME)
            .env("GIT_COMMITTER_EMAIL", BOT_EMAIL)
            .input(&message)
            .run();
        if !commit.success() {
            return fail_git(&commit, "could not commit the move");
        }
        let refspec = format!("{}:refs/heads/{BUMP_BRANCH}", commit.stdout.trim());
        let pushed = git.run(&["push", "-q", "origin", &refspec]);
        if !pushed.success() {
            return fail_git(
                &pushed,
                &format!(
                    "could not push the move to {BUMP_BRANCH}; the push needs the job's token with contents: write kept on the checkout's origin"
                ),
            );
        }
        for line in &lines {
            fail(&format!("pushed the move to {BUMP_BRANCH}: {line}"));
        }
        let server =
            env::var("GITHUB_SERVER_URL").unwrap_or_else(|_| "https://github.com".to_owned());
        let repository = env::var("GITHUB_REPOSITORY").unwrap_or_default();
        fail(&format!(
            "open its pull request: {server}/{repository}/compare/{BUMP_BRANCH}?expand=1"
        ));
        Status::Differs
    }
}
