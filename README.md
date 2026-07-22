# dotfiles

## Initial Setup

Run once on a new machine to drop `~/.dircolors`, `~/.config/git/ignore`, `~/.zprofile`, and `~/.zshenv` (each is overwritten with canonical content):

```shell
mise run setup:dotfiles
```

### Shell startup: `.zprofile` vs `.zshenv`

The Homebrew environment is split across two files on purpose:

- `.zprofile` runs `brew shellenv` to put `/opt/homebrew/bin` ahead of `/usr/bin` on PATH.
  - It must live in a login-shell file, because macOS `/etc/zprofile` runs `/usr/libexec/path_helper`, which rebuilds PATH from `/etc/paths` and demotes `/opt/homebrew/bin` to the end.
  - Only a `brew shellenv` running *after* path_helper re-prepends Homebrew, which is why it belongs here. See [Homebrew discussion #1127](https://github.com/orgs/Homebrew/discussions/1127).
- `.zshenv` only exports `HOMEBREW_PREFIX` and never touches PATH.
  - It runs for every shell, including non-login shells spawned by tools that do not inherit a login environment.
  - Such shells skip `.zprofile`, so without this they lack `HOMEBREW_PREFIX`, and the `$HOMEBREW_PREFIX`-expanding `ls` alias in `zshrc_fragment.sh` fails with `exit 127`.
  - Setting PATH here would be undone by path_helper, so only the variable is set.

## Tool Versions

Upgrade tools to the newest version released on or before a given date:

```shell
mise upgrade --before 7d --dry-run
mise upgrade --before 7d
mise upgrade --before 2024-06-01
```

## Agent Instructions

`~/.claude/CLAUDE.md` is a thin, machine-local entry point that imports the shareable and private parts separately:

- `.claude/INSTRUCTIONS.md` — user-scoped principles (shareable), symlinked to `~/.claude/INSTRUCTIONS.md`.
  - Deliberately not named `CLAUDE.md`, so it never loads as this repo's own project instructions.
- `~/.claude/RTK.md` — private and machine-local, not managed here (create it separately).
- `.claude/rules/` — path-scoped rules, loaded when Claude works with files matching each rule's `paths`.

Wire them once on a new machine:

```shell
ln -sf "$PWD/.claude/INSTRUCTIONS.md" ~/.claude/INSTRUCTIONS.md
ln -sfn "$PWD/.claude/rules" ~/.claude/rules
cat >~/.claude/CLAUDE.md <<'EOF'
@INSTRUCTIONS.md
@RTK.md
EOF
```

## Agent Skills

Most skills live under `.claude/skills/`, managed by [APM](https://github.com/microsoft/apm), co-located with the instructions and rules above.
GitHub Copilot [also reads `.claude/skills/`](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills), so this single directory serves both the primary Claude setup and Copilot as a secondary client.
`apm.yml` declares the packages and `apm.lock.yaml` pins the resolved commits and content hashes.

The `target: claude` field in `apm.yml` is deliberately the **singular** `target:` key, not the plural `targets:`.
`apm uninstall` reads only the singular field, so with `target:` set it honors the pin and touches `.claude/skills/` alone.
A plural `targets:` reads as unset, which makes uninstall auto-detect on-disk targets (`.github/`, `.cursor/`, …) and mirror skills into a stray `.agents/skills/`.

Symlink `~/.claude/skills` to it once:

```shell
ln -sfn "$PWD/.claude/skills" ~/.claude/skills
```

Restore pinned skills from `apm.lock.yaml`:

```shell
apm install
```

Add a new skill package (`owner/repo` for a single-skill repo, `owner/repo/path/to/skill` for a monorepo entry):

```shell
apm install owner/repo
```

Remove installed packages (also strips them from `apm.yml` and `apm.lock.yaml`):

```shell
mise run skills:remove <package> [more...]
```

Audit deployed files against the lockfile, plus integrity and hidden-character checks:

```shell
apm audit
```

Show packages whose upstream advanced past the pinned ref, then update:

```shell
apm outdated
apm install --update
```

### Gist-sourced skills

Some skills are published as a single-file GitHub gist, which APM deploys under a directory named after the gist hash rather than a readable name.
These are vendored from `gistSkills.json`, a `name -> raw gist URL` catalog, by `scripts/sync_gist_skills.sh`.
The catalog is the source of truth, and the materialized `.claude/skills/<name>/SKILL.md` is gitignored like APM deps.
Unlike APM packages, these are not restored by `apm install`; run `mise run skills:sync` separately.

List the catalog, then sync every entry (or one by name):

```shell
mise run skills:list
mise run skills:sync
mise run skills:sync japanese-tech-writing
```

Add a skill by putting a `name -> raw gist URL` entry in `gistSkills.json`, then run `mise run skills:sync <name>`.

Since the materialized `SKILL.md` is gitignored and `skills:sync` overwrites it Gist→local, the gist is the only durable copy of a skill's content.
After editing a skill locally, push the change back to its gist, or the next sync silently discards it:

```shell
mise run skills:push japanese-tech-writing
```

This writes to the gist through the GitHub API (`gh` auth required) and verifies the result, since `gh gist edit` silently no-ops in a non-interactive shell.

### Repo-tracked skills

A few skills are written here rather than pulled from an upstream, and this repository is their only copy.
`.gitignore` excludes all of `.claude/skills/*`, which is what keeps APM and gist output out of version control.
A hand-written skill therefore needs one line to unignore it:

```gitignore
!/.claude/skills/<name>/
```

Re-including the directory is enough — the exclusion above uses a single `*`, which does not cross `/`, so it never matched the contents in the first place.

No sync step follows: `~/.claude/skills` is a symlink to this directory, so the skill is live as soon as the files exist.
`ask-kboat`, described under [Basic Memory projects](#basic-memory-projects), is one of these.

## MCP Servers

The only stdio MCP server in use is `basic-memory`, already configured in Claude Desktop and Claude Code.
Claude Desktop's DXT extensions and remote connectors are managed in-app, not from this directory.

To wire `basic-memory` into a fresh client:

```shell
claude mcp add-json -s user basic-memory '{"command":"uvx","args":["basic-memory","mcp"]}'
```

### Basic Memory projects

A knowledge base is a [Basic Memory *project*](https://github.com/basicmachines-co/basic-memory#readme): a name bound to a directory of Markdown notes.
Every tool takes the project name, so a base has to be registered before any client can reach it — wiring the server above is not enough on its own.

```shell
basic-memory project add <name> <path>
basic-memory project list
```

Registration is per-machine, so a fresh client needs the bases it reads registered by hand. The one this repository's skills depend on:

```shell
basic-memory project add k-boat-knowledge ~/Documents/_repos/my-foam/.kboat
```

The notes are plain Markdown and stay readable in Obsidian or Foam without the server, which is only the search layer.
So the notes directory is worth version-controlling in its own right, independently of this registration, which is local client state.

Several projects are registered; the `memory-*` skills work against whichever one is in play.
The one this repository names directly is `k-boat-knowledge`, the distilled side of [K-Boat](https://github.com/irisTa56/k-boat) — a skill package that reads sources through NotebookLM and matures them into a concept graph.
K-Boat owns the writing side; this repository only reads that base, through the repo-tracked `ask-kboat` skill, which answers a question from the concept notes and keeps what the base says distinct from general knowledge.
