# Keeping research recoverable with Entire

[Entire](https://github.com/entireio/cli) records agent sessions as checkpoints under git refs and adds an `Entire-Checkpoint` trailer to each commit a session makes, so `entire why <file>:<line>` can lead from a line back to the conversation that produced it.
A repository uses it when it has an `.entire/` directory.
These points decide whether the research behind a decision can be found again.

## A checkpoint needs a commit

- A checkpoint is created when the session commits to the repository itself, so research whose session never commits there is not recorded, and a commit in the private workspace does not make one.
  - Commit something to the repository in the same session once the research has produced its result, even if it is only the ADR or the plan.
  - If nothing is ready to commit, have the session make an empty commit (`git commit --allow-empty`), which still gets a checkpoint.
- The checkpoint stores the session's transcript, including what the Write and Edit tools wrote.
  - Files kept in the private workspace are therefore still recoverable from the checkpoint, provided they were written with those tools rather than through the shell.
- Files under `.claude/` are left out of a checkpoint's file list by design, because Entire treats each agent's own configuration directory as protected (`ProtectedDirs` in [`cmd/entire/cli/agent/agent.go`](https://github.com/entireio/cli/blob/main/cmd/entire/cli/agent/agent.go)).
  - Keep research notes outside it.

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
Where the repository is public, its checkpoints go to its companion repository ([private-workspace.md](private-workspace.md)) and never to `origin`, and the committed settings do not name that repository:

- **Committed**: `.entire/settings.json` sets `strategy_options.checkpoint_push_remote` to `checkpoints`, the name of a git remote.
  - That setting is fail-closed, so a clone without the remote, such as a fork's, sends checkpoints nowhere.
  - `checkpoint_remote` is not used, since it takes a repository's name rather than a remote's, and a clone that inherits it without owning that repository [falls back to pushing checkpoints to `origin`](https://github.com/entireio/cli#checkpoint-remote).
- **In each clone's git config**, which linked worktrees share:
  - `git remote add checkpoints <URL of the companion repository>`.
  - `git config remote.checkpoints.push 'refs/entire/checkpoints/*:refs/entire/checkpoints/*'`, so that a push to that remote sends the checkpoint refs and no branch.
  - `git config hook.checkpoints-sync.event pre-push` and `git config hook.checkpoints-sync.command 'test "$1" = checkpoints || git push --quiet checkpoints #'`, so that a push to any other remote sends them as well.
    - Entire by itself sends checkpoints only on a push to the remote that setting names.
    - Git runs the command with the pushed remote's name as `$1` and appends its arguments, which the trailing `#` drops ([git-hook](https://git-scm.com/docs/git-hook)).

Before researching anything that should not be public, check that `entire status` reports checkpoints syncing to `checkpoints`, and set the clone up if it does not.
