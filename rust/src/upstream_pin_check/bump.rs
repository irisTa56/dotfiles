//! Under GitHub Actions, moving the differing pins on origin and proposing the move as a
//! pull request.

use std::collections::HashMap;
use std::fmt::Write;

use dotfiles_shared::Failure;

use super::{
    Differing, FailLine, GitHub, Marker, Message, Move, Moves, Proposal, PullRequest, Scratch,
    Status, Text,
};
use crate::git::Run;

/// The status `git ls-remote --exit-code` exits with where no ref matches.
const LS_REMOTE_NO_REF: i32 = 2;
/// The move's commit subject, and its pull request's title.
const TITLE: &str = "chore: move upstream pins to their default branches";
const BOT_NAME: &str = "github-actions[bot]";
const BOT_EMAIL: &str = "41898282+github-actions[bot]@users.noreply.github.com";

/// The default branch's tip of each differing pin's url and path.
type Tips<'d> = HashMap<(&'d str, &'d str), &'d str>;

impl Scratch {
    /// In GitHub Actions the checkout is whatever the run was for, often a pull request's
    /// merge commit, so the move is made on the repository's default branch instead,
    /// pushed to a branch of its own, and proposed as a pull request into that branch.
    /// While that branch exists, its pull request is open, so the move is made on the
    /// branch instead, over whatever a person added there.
    pub(super) fn move_on_origin(
        &self,
        differing: &Differing,
        proposal: Proposal,
    ) -> Result<Status, Failure> {
        if self.bump_branch_exists(proposal.branch)? {
            self.move_on_bump_branch(differing, proposal.branch)
        } else {
            self.propose_move(differing, proposal)
        }
    }

    /// Whether `branch` exists on origin; a run that cannot tell ends here.
    fn bump_branch_exists(&self, branch: &str) -> Result<bool, Failure> {
        let bump_ref = format!("refs/heads/{branch}");
        let exists = self
            .git
            .run(&["ls-remote", "-q", "--exit-code", "origin", &bump_ref]);
        if exists.success() {
            return Ok(true);
        }
        if exists.status() == Some(LS_REMOTE_NO_REF) {
            return Ok(false);
        }
        Err(
            FailLine(&format!("could not tell whether {branch} exists on origin"))
                .ending_git(&exists),
        )
    }

    /// Moves the pins on the default branch, pushes the move to the proposal's branch,
    /// new, and opens its pull request, as a draft where the proposal says so. What opening
    /// one needs is checked before the push, so that a run lacking it leaves no branch
    /// behind.
    fn propose_move(&self, differing: &Differing, proposal: Proposal) -> Result<Status, Failure> {
        let branch = proposal.branch;
        let github = GitHub::from_env()?;
        let default_branch = self.default_branch()?;
        let base = self.fetch_from_origin("HEAD")?;
        let (tree, moved) = self.stage_moves(&base, "origin's default branch", differing)?;
        if moved.0.is_empty() {
            return Err(FailLine("origin's default branch holds none of these pins to move, or holds them where the path matches the tip's already").ending());
        }
        let message = Message::of(&moved);
        self.push_move(&base, &tree, &message, branch)?;
        for line in &message.0 {
            FailLine(&format!("pushed the move to {branch}: {line}")).report();
        }
        self.open_pull_request(&github, &default_branch, &message, proposal)?;
        Ok(Status::Differs)
    }

    /// Moves the pins as `branch` spells them, in a commit over its tip, which its open
    /// pull request then shows; a branch whose pins are at the tips already is left as it
    /// is.
    fn move_on_bump_branch(&self, differing: &Differing, branch: &str) -> Result<Status, Failure> {
        let base = self.fetch_from_origin(&format!("refs/heads/{branch}"))?;
        let (tree, moved) = self.stage_moves(&base, branch, differing)?;
        if moved.0.is_empty() {
            FailLine(&format!(
                "{branch} holds none of these pins to move, or holds them where the path matches the tip's already, so it is left as it is"
            ))
            .report();
            return Ok(Status::Differs);
        }
        let message = Message::of(&moved);
        self.push_move(&base, &tree, &message, branch)?;
        for line in &message.0 {
            FailLine(&format!("pushed the move onto {branch}: {line}")).report();
        }
        Ok(Status::Differs)
    }

    /// The name of origin's default branch, which the pull request goes into.
    fn default_branch(&self) -> Result<String, Failure> {
        let listed = self
            .git
            .run(&["ls-remote", "-q", "--symref", "origin", "HEAD"]);
        let name = listed.stdout().lines().find_map(|line| {
            let (target, name) = line.strip_prefix("ref: refs/heads/")?.split_once('\t')?;
            (name == "HEAD").then(|| target.to_owned())
        });
        name.ok_or_else(|| {
            FailLine("could not tell which branch is origin's default").ending_git(&listed)
        })
    }

    /// Fetches `rev` from origin and returns the commit it named. A shallow checkout, as
    /// actions/checkout leaves by default, takes that commit alone; a full one fetches its
    /// history whole, since a depth would leave it shallow for the steps after this one.
    fn fetch_from_origin(&self, rev: &str) -> Result<String, Failure> {
        let git = &self.git;
        let shallow = git.run(&["rev-parse", "--is-shallow-repository"]);
        if !shallow.success() {
            return Err(
                FailLine("could not tell whether the checkout is shallow").ending_git(&shallow)
            );
        }
        let fetched = if shallow.stdout().trim() == "true" {
            git.run(&["fetch", "-q", "--depth=1", "origin", rev])
        } else {
            git.run(&["fetch", "-q", "origin", rev])
        };
        if !fetched.success() {
            return Err(
                FailLine(&format!("could not fetch {rev} from origin")).ending_git(&fetched)
            );
        }
        Ok(git
            .run(&["rev-parse", "FETCH_HEAD"])
            .stdout()
            .trim()
            .to_owned())
    }

    /// Builds the tree of the move without touching the working tree or its index: a
    /// scratch index over the default branch's tree, with each moved file swapped in.
    fn stage_moves(
        &self,
        base: &str,
        place: &str,
        differing: &Differing,
    ) -> Result<(String, Moves), Failure> {
        let tips: Tips = differing
            .0
            .iter()
            .map(|Move { pin, tip }| ((pin.url.as_str(), pin.path.as_str()), tip.as_str()))
            .collect();
        let read = self.indexed(&["read-tree", base]);
        if !read.success() {
            return Err(FailLine(&format!("could not read {place}")).ending_git(&read));
        }
        let mut moved = Moves::default();
        for file in differing.files() {
            moved.0.extend(self.stage_file(base, place, file, &tips)?.0);
        }
        let tree = self.indexed(&["write-tree"]).stdout().trim().to_owned();
        Ok((tree, moved))
    }

    /// Moves the pins in `file` as the default branch spells them, and stages the result at
    /// the mode the file has there; a file the default branch lacks, or whose pins none
    /// move, is left out.
    fn stage_file(
        &self,
        base: &str,
        place: &str,
        file: &str,
        tips: &Tips,
    ) -> Result<Moves, Failure> {
        let git = &self.git;
        let listed = git.run(&["ls-tree", base, "--", file]);
        if !listed.success() {
            return Err(FailLine(&format!("could not read {file} on {place}")).ending_git(&listed));
        }
        let entry = listed.stdout();
        let fields: Vec<&str> = entry.split_whitespace().collect();
        let [mode, _, _, ..] = fields[..] else {
            return Ok(Moves::default());
        };
        let before = git.run(&["cat-file", "blob", &format!("{base}:{file}")]);
        if !before.success() {
            return Err(FailLine(&format!("could not read {file} on {place}")).ending_git(&before));
        }
        let (after, moved) =
            Text(before.stdout_bytes()).rewrite(|found| self.base_tip(tips, found));
        if moved.0.is_empty() {
            return Ok(moved);
        }
        let blob = git
            .call(&["hash-object", "-w", "--stdin"])
            .input(&after)
            .run()
            .stdout()
            .trim()
            .to_owned();
        let update = self.indexed(&[
            "update-index",
            "--cacheinfo",
            &format!("{mode},{blob},{file}"),
        ]);
        if !update.success() {
            return Err(
                FailLine(&format!("could not stage the move of {file}")).ending_git(&update)
            );
        }
        Ok(moved)
    }

    /// The tip a pin the default branch spells moves to. The default branch may pin
    /// another commit than the checkout, or lack a pin the checkout has, so a pin there
    /// moves only where its own path differs from the tip's.
    fn base_tip(&self, tips: &Tips, found: &Marker) -> Option<String> {
        let tip = *tips.get(&(found.url, found.path))?;
        if tip == found.commit {
            return None;
        }
        if !self.holds(found.url, found.commit) {
            return Some(tip.to_owned());
        }
        let [pinned_tree, tip_tree] = self.trees_at([found.commit, tip], found.path);
        (pinned_tree != tip_tree).then(|| tip.to_owned())
    }

    /// Runs git on the scratch index the move is staged in.
    fn indexed(&self, args: &[&str]) -> Run {
        let index_file = self.dir().join("bump-index");
        self.git
            .call(args)
            .env("GIT_INDEX_FILE", &index_file.to_string_lossy())
            .run()
    }

    /// Commits `tree` over `base` as the bot and pushes it to `branch`.
    fn push_move(
        &self,
        base: &str,
        tree: &str,
        message: &Message,
        branch: &str,
    ) -> Result<(), Failure> {
        let git = &self.git;
        let commit = git
            .call(&["commit-tree", tree, "-p", base, "-F", "-"])
            .env("GIT_AUTHOR_NAME", BOT_NAME)
            .env("GIT_AUTHOR_EMAIL", BOT_EMAIL)
            .env("GIT_COMMITTER_NAME", BOT_NAME)
            .env("GIT_COMMITTER_EMAIL", BOT_EMAIL)
            .input(message.text().as_bytes())
            .run();
        if !commit.success() {
            return Err(FailLine("could not commit the move").ending_git(&commit));
        }
        let refspec = format!("{}:refs/heads/{branch}", commit.stdout().trim());
        let pushed = git.run(&["push", "-q", "origin", &refspec]);
        if !pushed.success() {
            return Err(FailLine(&format!(
                    "could not push the move to {branch}; the push needs the job's token with contents: write kept on the checkout's origin"
                )).ending_git(&pushed));
        }
        Ok(())
    }

    /// Opens the pull request of the proposal's branch into `base`, as a draft where the
    /// proposal says so. Where it cannot, it says why at once, before a push that may stall,
    /// and deletes the branch this run pushed again, since a run that finds the branch takes
    /// its pull request to be open and opens none; the run ends on how that went.
    fn open_pull_request(
        &self,
        github: &GitHub,
        base: &str,
        message: &Message,
        proposal: Proposal,
    ) -> Result<(), Failure> {
        let body = message.list();
        let pull = PullRequest {
            head: proposal.branch,
            base,
            title: TITLE,
            body: &body,
            draft: proposal.draft,
        };
        match github.open(self.dir(), &pull) {
            Ok(page) => {
                FailLine(&format!("opened its pull request: {page}")).report();
                Ok(())
            }
            Err(reason) => {
                FailLine(&format!("could not open its pull request: {reason}")).report();
                Err(self.withdraw_push(proposal.branch))
            }
        }
    }

    /// Deletes `branch` from origin, where this run pushed it, and returns what ends the
    /// run, saying how that went.
    fn withdraw_push(&self, branch: &str) -> Failure {
        let deleted = self
            .git
            .run(&["push", "-q", "origin", &format!(":refs/heads/{branch}")]);
        if deleted.success() {
            FailLine(&format!(
                "deleted {branch} again, so that the next run opens its pull request"
            ))
            .ending()
        } else {
            FailLine(&format!("could not delete {branch}; delete it by hand, or the next run will open no pull request")).ending_git(&deleted)
        }
    }
}

impl Message {
    pub(super) fn of(moved: &Moves) -> Self {
        Message(moved.describe())
    }

    /// A line per move, as a Markdown list: the commit's body and the pull request's.
    fn list(&self) -> String {
        self.0.iter().fold(String::new(), |mut list, line| {
            let _ = writeln!(list, "- {line}");
            list
        })
    }

    fn text(&self) -> String {
        format!("{TITLE}\n\n{}", self.list())
    }
}
