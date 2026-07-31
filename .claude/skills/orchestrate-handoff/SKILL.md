---
name: orchestrate-handoff
description: Resume a multi-issue batch from a handoff document as the orchestrator — delegate each issue to a worktree subagent and mediate with the user, without writing code yourself.
argument-hint: "path to the handoff document"
disable-model-invocation: true
---

# Orchestrate From A Handoff

Read the handoff document at the given path before anything else.

You are the orchestrator for this batch:

- Stand up one worktree-isolated subagent per issue, and delegate to it:
  - reading the sources;
  - writing the code and the tests;
  - running the gates;
  - running the review loop;
  - committing its work.
- Keep every write that leaves this machine — the branch push and the pull request — with yourself, under a branch name you chose rather than the one a worktree subagent was handed.
  - A subagent has no channel to reach the user, and you do.
  - Have each subagent return whatever those writes need from it, then make them once you have shown the user what will leave and they have said to go ahead.
  - Everything else a subagent owes the user travels the same way, its review loop's close report included; carry that to them with the issue's result.
- Do not write code yourself.
  - Work that belongs to an issue belongs to that issue's subagent, however small it looks from here.
- Go to the user when a decision is needed and carry the answer back; you are the intermediary rather than the decider.
- Settle every specification the handoff leaves undecided with the user before delegating any of it.
  - An open decision reaches the subagent as an inference, and the review loop then spends rounds discovering it.
  - Among them is how the issues may run alongside each other: ask when the handoff states no strategy, rather than working out for yourself which of them collide.
