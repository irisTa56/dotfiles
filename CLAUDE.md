# CLAUDE.md

## Project Guidelines

- This file loads only for sessions working inside this repository, so a rule that should hold in every project goes in `.claude/INSTRUCTIONS.md`, or in `.claude/rules/` when it is path-scoped.
- `~/.claude/skills`, `~/.claude/rules`, and `~/.claude/INSTRUCTIONS.md` are symlinks into this repository, so editing what they point at changes agent behavior in every project rather than only here.
  - A worktree of this repository is not what those symlinks point at, so a skill edited there is not the skill running.
- Only the skill directories that `.gitignore` unignores are tracked here, so an edit to any other exists on this machine alone and no clone carries it.
  - A gist-sourced skill lives in its gist, so send an edit back with `mise run skills:push <name>`, an external write, before the next `mise run skills:sync` overwrites it.
- Checks: `mise run pre-commit` from this repository's root; the parent workspace defines a smaller task of the same name.
  - It leaves out the `md-to-docx` skill's tests, which are the required CI check: run `npm ci && npm test` there when its scripts, fixtures, or dependencies change.
  - Pass `rumdl` no path; an explicit `.claude/skills/*/SKILL.md` glob sweeps the vendored copies that `.gitignore` holds out of the default run.
- PR bodies are English prose, and there is no template to fill in.
- The squashed subject on `main` is the commit's when the PR has one commit and the PR title when it has more, so write both as Conventional Commits.
