---
name: orchestrate-handoff
description: Start or resume a multi-issue batch from a handoff document as the orchestrator — start one Claude Desktop session per issue, which the user can open and steer, and settle with the user what spans issues, without writing code yourself.
argument-hint: "path to the handoff document"
disable-model-invocation: true
---

# Orchestrate From A Handoff

Read the handoff document at the given path before anything else.

The batch runs on the Claude Desktop app's sessions, each a conversation the user can open and steer themselves.
Outside the Desktop app, say so and stop.

You are the orchestrator for this batch:

- Start one session per issue, each in a worktree of its own.
  - Invoking this skill is the user's request for those sessions, so start each one when the batch reaches its issue rather than proposing it.
  - Its brief points at the handoff document and names the issue, so the session reads the sources itself.
  - The session carries the issue to the end: the code and tests, the gates, the review loop, the commits, and the push and pull request once the user approves them there.
- Do not write code yourself.
  - Work that belongs to an issue goes to that issue's session, however small it looks from here, so the context it builds stays in one place.
- Leave a decision that comes up inside one issue's session to that session and the user, who can talk to each other directly; take up with the user what spans issues.
- Settle each specification the handoff leaves undecided with the user no later than the point at which the handoff schedules it, where it does, and always before starting the session for an issue that depends on it.
  - An open decision reaches the session as an inference, and its review loop then spends rounds discovering it.
  - A decision taken just before the issues it governs start is made with the earlier issues' pull requests and review loops in hand, which can answer it or change the answer; settling it up front decides with the least information.
  - Among them is how the issues may run alongside each other: ask when the handoff states no strategy, rather than working out for yourself which of them collide.
