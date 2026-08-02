---
description: Keep agent instruction and rule files necessary and sufficient
paths:
  - "**/AGENTS.md"
  - "**/CLAUDE.md"
  - "**/INSTRUCTIONS.md"
  - "**/.claude/rules/*.md"
---

# Agent Instruction Files

Follow these rules when writing or editing `CLAUDE.md`, `AGENTS.md`, and the rule files beside them — anything that loads into a session's context rather than being read on demand.

## Core principle

- These files load without being asked for — on every turn, or on every read of a path they scope to — so every line costs context budget repeatedly.
- Include only what changes agent behavior and cannot be obtained otherwise. When in doubt, leave it out.
- This is not just a token-cost heuristic: a study of 2,500 repositories found that [instruction files duplicating repo content lowered agent success and raised cost](https://github.blog/ai-and-ml/github-copilot/how-to-write-a-great-agents-md-lessons-from-over-2500-repositories/), while non-obvious human-written context helped.

## What belongs

- Project-specific conventions an agent cannot infer from the code (e.g. naming schemes, required file headers, domain rules).
- Constraints and expectations that override the agent's default behavior.
- A one-line pointer to the source-of-truth file for anything detailed.

## What to leave out

- Generic best practices the agent already follows by default (e.g. "write clean code", "add tests").
- Facts the agent can derive on demand — directory contents from a listing, build and test commands from the project's task config — unless a one-line pointer saves repeated rediscovery.
- Rules a linter, formatter, or config file already enforces; let the toolchain carry them.
- Content that is authoritative elsewhere. Link to it instead of restating; a duplicated copy drifts out of sync.
- Near-identical wording in two places. Say it once, in the most relevant section.
- Historical narrative — how a convention evolved, migration backstory, or changelog-style entries. State the current rule; let git history carry the past.
- Anything pointing the same direction as the harness's own defaults. An instruction earns its place by overriding a default, not by agreeing with one.
- Either side of a contradiction between two instruction files. Cut the one whose scope fits worse; [two files giving different guidance leave Claude picking one arbitrarily](https://code.claude.com/docs/en/memory).

## Form

- Write concrete, verifiable instructions over vague ones — "use 2-space indentation" beats "format code properly". This bans the uncheckable, not the general.
- Keep an always-loaded file at the level of the rule, and let a path-scoped rule or a skill carry the specifics. Piling on detail for robustness is how such a file turns redundant.
- Keep each instruction to a single scannable bullet (see `document-writing.md`).
- Prefer describing where things live and the shape of the project over an exhaustive structure dump; let the agent discover specifics on demand.
- Keep each file small. [Anthropic targets under ~200 lines per file](https://code.claude.com/docs/en/memory); when one grows past that, split by path scope using `paths` frontmatter rather than appending.

## Maintain

- Review periodically and prune entries that went stale or have migrated into the toolchain.
- For maintainer notes that should not cost context, use block-level HTML comments — Claude Code strips them before loading the file.
