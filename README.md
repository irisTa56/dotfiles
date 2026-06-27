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

## Agent Skills

Most skills live under `.agents/skills/`, managed by [APM](https://github.com/microsoft/apm).
The `targets: [agent-skills]` field in `apm.yml` makes APM deploy skills to the shared
cross-client `.agents/skills/` directory instead of per-client paths like `.claude/skills/`.
`apm.yml` declares the packages and `apm.lock.yaml` pins the resolved commits and content hashes.
Symlink `~/.claude/skills` to it once:

```shell
ln -sfn "$PWD/.agents/skills" ~/.claude/skills
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
The catalog is the source of truth, and the materialized `.agents/skills/<name>/SKILL.md` is gitignored like APM deps.
Unlike APM packages, these are not restored by `apm install`; run `mise run skills:sync` separately.

List the catalog, then sync every entry (or one by name):

```shell
mise run skills:list
mise run skills:sync
mise run skills:sync japanese-tech-writing
```

Add a skill by putting a `name -> raw gist URL` entry in `gistSkills.json`, then run `mise run skills:sync <name>`.

## MCP Servers

The only stdio MCP server in use is `basic-memory`, already configured in Claude Desktop and Claude Code.
Claude Desktop's DXT extensions and remote connectors are managed in-app, not from this directory.

To wire `basic-memory` into a fresh client:

```shell
claude mcp add-json -s user basic-memory '{"command":"uvx","args":["basic-memory","mcp"]}'
```
