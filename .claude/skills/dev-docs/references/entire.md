# Keeping research recoverable with Entire

[Entire](https://github.com/entireio/cli) records agent sessions as checkpoints under git refs and adds an `Entire-Checkpoint` trailer to each commit a session makes, so `entire why <file>:<line>` can lead from a line back to the conversation that produced it.
A repository uses it when it has an `.entire/` directory.
These points decide whether the research behind a decision can be found again.

## A checkpoint needs a commit

- A checkpoint is created when the session commits to the repository itself, so research whose session never commits there is not recorded, and a commit in the private workspace does not make one.
  - Commit something to the repository in the same session once the research has produced its result, even if it is only the ADR or the plan.
  - If nothing is ready to commit, have the session make an empty commit (`git commit --allow-empty`), which still gets a checkpoint.

## Subagents

- A subagent's own transcript is recorded only when the Agent call passes `run_in_background: true` explicitly, although Claude Code backgrounds subagents without it ([entireio/cli#2556](https://github.com/entireio/cli/issues/2556)).
  - Pass it on every Agent call you want recorded.
- With fork mode on, Claude Code removes that parameter from the Agent tool, so no subagent can be recorded ([fork mode](https://code.claude.com/docs/en/sub-agents#turn-fork-mode-on-or-off)).
  - Fork mode is on by default in interactive sessions, and off with `-p` and in the Agent SDK.
  - A repository that wants subagents recorded sets `CLAUDE_CODE_FORK_SUBAGENT=0`.
  - If the Agent tool offers no `run_in_background` parameter, tell the person instead of assuming the research will be kept.
- A subagent's final report reaches the parent transcript either way, so notes that carry each conclusion and its source keep what a decision rests on even when a subagent's working is lost.
- The rules on `run_in_background` and fork mode hold for every session in the repository, not only for research, so they belong in its instructions (`CLAUDE.md`, `CLAUDE.local.md`, or the like).
  - If the instructions lack them, propose adding them.

## Finding the conversation after a squash merge

- A squash merge gathers every commit's trailer into one commit, and `entire why` on the main branch then picks the first checkpoint that touched the file, whichever commit added the line ([`attribution.go`](https://github.com/entireio/cli/blob/main/cmd/entire/cli/attribution.go)).
  - It is therefore reliable only down to the file.
- To reach the right conversation, fetch the pull request's head (`git fetch origin pull/<N>/head`), check it out in a temporary worktree, and run `entire why` there on the matching line.
  - GitHub keeps `refs/pull/<N>/head` after the branch is deleted.

## Visibility

Checkpoints hold prompts, responses, and file contents verbatim, and Entire's own redaction is best-effort ([security and privacy](https://github.com/entireio/cli/blob/main/docs/security-and-privacy.md)).
The repository sends them to its companion repository ([private-workspace.md](private-workspace.md)) and never to `origin`, and no file it commits names that repository:

- **Committed**: `.entire/settings.json` sets `strategy_options.checkpoint_push_remote` to `checkpoints`, the name of a git remote, [which is fail-closed](https://github.com/entireio/cli#checkpoint-remote).
- **In each clone**: `git remote add checkpoints <URL of the companion repository>`, and two untracked files in the main checkout.
  - `.entire/settings.local.json` sets `strategy_options.checkpoint_remote` to the companion repository's provider and name.
  - `.worktreeinclude` lists `.entire/settings.local.json`, which Claude Code [copies into each worktree it creates](https://code.claude.com/docs/en/worktrees#copy-gitignored-files-into-worktrees).
    - It stays out of the repository, listed in `.git/info/exclude`, because it has no per-person counterpart and each person lists files of their own in it.

Before researching, run `entire status` in the working tree you are in, which must print `Checkpoints sync to: dedicated checkpoint remote (<owner>/<name>)`.
`Checkpoints sync to: checkpoints (set by checkpoint_push_remote)` is not that: it is a working tree without its `.entire/settings.local.json`, from which a push to `origin` carries no checkpoint.
Where it prints anything else, supply what is missing and run it again, putting to the person any change to a committed file, or to the main checkout from a worktree, [where Claude Code refuses your writes](https://code.claude.com/docs/en/worktrees#how-claude-code-enforces-isolation).
