---
description: Keep always-loaded agent instruction files necessary and sufficient
paths:
  - "**/AGENTS.md"
  - "**/CLAUDE.md"
---

# Agent Instruction Files

Follow these rules when writing or editing `CLAUDE.md`, `AGENTS.md`, and other files that load into every session's context.

## Core principle

- These files are loaded on every turn, so every line costs context budget continuously.
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

## Form

- Write concrete, verifiable instructions over vague ones — "use 2-space indentation" beats "format code properly".
- Keep each instruction to a single scannable bullet (see `readable-writing.md`).
- Prefer describing where things live and the shape of the project over an exhaustive structure dump; let the agent discover specifics on demand.
- Keep each file small. [Anthropic targets under ~200 lines per file](https://code.claude.com/docs/en/memory); when one grows past that, split by path scope using `paths` frontmatter rather than appending.

## Maintain

- Review periodically and prune entries that went stale, contradict another rule, or have migrated into the toolchain.
- For maintainer notes that should not cost context, use block-level HTML comments — Claude Code strips them before loading the file.
