//! Under GitHub Actions, moving the differing pins on origin's default branch and pushing
//! the move to a branch of its own.

use std::collections::HashMap;
use std::env;

use super::{Differing, Failure, Marker, Move, Moves, Scratch, Status, Text};

/// Once this branch exists it is the person's who opens its pull request, who may add
/// commits to it, so a later run leaves it alone; merging that pull request deletes it where
/// the repository deletes merged branches, and the next run pushes afresh.
const BUMP_BRANCH: &str = "upstream-pin-bump";
/// The status `git ls-remote --exit-code` exits with where no ref matches.
const LS_REMOTE_NO_REF: i32 = 2;
const BOT_NAME: &str = "github-actions[bot]";
const BOT_EMAIL: &str = "41898282+github-actions[bot]@users.noreply.github.com";

impl Scratch {
    /// In GitHub Actions the checkout is whatever the run was for, often a pull request's
    /// merge commit, so the move is made on the repository's default branch instead and
    /// pushed to a branch of its own. A person opens the pull request from it: opening one
    /// with the job's token needs a repository setting that also lets that token approve
    /// pull requests.
    pub(super) fn move_on_default_branch(&self, differing: &Differing) -> Status {
        let git = &self.git;
        let bump_ref = format!("refs/heads/{BUMP_BRANCH}");
        let exists = git.run(&["ls-remote", "-q", "--exit-code", "origin", &bump_ref]);
        if exists.success() {
            Failure(&format!(
                "{BUMP_BRANCH} already exists on origin, so it is left as it is"
            ))
            .report();
            return Status::Differs;
        }
        if exists.status() != Some(LS_REMOTE_NO_REF) {
            return Failure(&format!(
                "could not tell whether {BUMP_BRANCH} exists on origin"
            ))
            .report_git(&exists);
        }

        // A shallow checkout, as actions/checkout leaves by default, takes the default
        // branch's tip alone; a full one fetches it whole, since a depth would leave it
        // shallow for the steps after this one.
        let shallow = git.run(&["rev-parse", "--is-shallow-repository"]);
        if !shallow.success() {
            return Failure("could not tell whether the checkout is shallow").report_git(&shallow);
        }
        let fetched = if shallow.stdout().trim() == "true" {
            git.run(&["fetch", "-q", "--depth=1", "origin", "HEAD"])
        } else {
            git.run(&["fetch", "-q", "origin", "HEAD"])
        };
        if !fetched.success() {
            return Failure("could not fetch origin's default branch").report_git(&fetched);
        }
        let base = git
            .run(&["rev-parse", "FETCH_HEAD"])
            .stdout()
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
    fn stage_moves(&self, base: &str, differing: &Differing) -> Result<(String, Moves), Status> {
        let git = &self.git;
        let tips: HashMap<(&str, &str), &str> = differing
            .0
            .iter()
            .map(|Move { pin, tip }| ((pin.url.as_str(), pin.path.as_str()), tip.as_str()))
            .collect();
        let tip_for = |found: &Marker| {
            let tip = *tips.get(&(found.url, found.path))?;
            if tip == found.commit {
                return None;
            }
            let Ok(pinned) = self.fetch(found.url, found.commit) else {
                return Some(tip.to_owned());
            };
            (self.tree_at(&pinned, found.path) != self.tree_at(tip, found.path))
                .then(|| tip.to_owned())
        };
        let index_file = self.dir().join("bump-index");
        let index_file = index_file.to_string_lossy();
        let indexed = |args: &[&str]| git.call(args).env("GIT_INDEX_FILE", &index_file).run();
        let read = indexed(&["read-tree", base]);
        if !read.success() {
            return Err(Failure("could not read origin's default branch").report_git(&read));
        }
        let mut moved = Moves::default();
        for file in differing.files() {
            let listed = git.run(&["ls-tree", base, "--", file]);
            if !listed.success() {
                return Err(
                    Failure(&format!("could not read {file} on origin's default branch"))
                        .report_git(&listed),
                );
            }
            let entry = listed.stdout();
            let fields: Vec<&str> = entry.split_whitespace().collect();
            let [mode, _, _, ..] = fields[..] else {
                continue;
            };
            let before = git.run(&["cat-file", "blob", &format!("{base}:{file}")]);
            if !before.success() {
                return Err(
                    Failure(&format!("could not read {file} on origin's default branch"))
                        .report_git(&before),
                );
            }
            let (after, file_moved) = Text(before.stdout_bytes()).rewrite(tip_for);
            if file_moved.0.is_empty() {
                continue;
            }
            let blob = git
                .call(&["hash-object", "-w", "--stdin"])
                .input(&after)
                .run()
                .stdout()
                .trim()
                .to_owned();
            let update = indexed(&[
                "update-index",
                "--cacheinfo",
                &format!("{mode},{blob},{file}"),
            ]);
            if !update.success() {
                return Err(
                    Failure(&format!("could not stage the move of {file}")).report_git(&update)
                );
            }
            moved.0.extend(file_moved.0);
        }
        if moved.0.is_empty() {
            Failure("origin's default branch holds none of these pins to move, or holds them where the path matches the tip's already").report();
            return Err(Status::Unknown);
        }
        let tree = indexed(&["write-tree"]).stdout().trim().to_owned();
        Ok((tree, moved))
    }

    /// Commits `tree` over `base` as the bot and pushes it to [`BUMP_BRANCH`].
    fn push_move(&self, base: &str, tree: &str, moved: &Moves) -> Status {
        let git = &self.git;
        let lines = moved.describe();
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
            .input(message.as_bytes())
            .run();
        if !commit.success() {
            return Failure("could not commit the move").report_git(&commit);
        }
        let refspec = format!("{}:refs/heads/{BUMP_BRANCH}", commit.stdout().trim());
        let pushed = git.run(&["push", "-q", "origin", &refspec]);
        if !pushed.success() {
            return Failure(&format!(
                    "could not push the move to {BUMP_BRANCH}; the push needs the job's token with contents: write kept on the checkout's origin"
                )).report_git(&pushed);
        }
        for line in &lines {
            Failure(&format!("pushed the move to {BUMP_BRANCH}: {line}")).report();
        }
        let server =
            env::var("GITHUB_SERVER_URL").unwrap_or_else(|_| "https://github.com".to_owned());
        let repository = env::var("GITHUB_REPOSITORY").unwrap_or_default();
        Failure(&format!(
            "open its pull request: {server}/{repository}/compare/{BUMP_BRANCH}?expand=1"
        ))
        .report();
        Status::Differs
    }
}
