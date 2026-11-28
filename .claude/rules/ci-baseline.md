---
description: The CI baseline for irisTa56's public repositories
paths:
  - "**/.github/workflows/*.yml"
  - "**/.github/dependabot.yml"
---

# CI baseline

Hold a public repository owned by irisTa56 to this baseline when writing or editing its CI.
Elsewhere, a repository's own conventions come first, and this is only a reference.
A repository departs from it only where a comment in its workflow says why.

## Checks

- One file lists the repository's checks, and the commit hook and CI both run them from it, so what a check runs is said in one place.
  - In a repository that uses hk, the file is `hk.pkl`, whose `check` hook holds every check. A check that reads files one at a time is a step of its own, and a check of the whole project is a mise task a step calls. CI runs each step with `hk check --all --step <name>`.
  - In one that does not, the file is `mise.toml`. `pre-commit` depends on `qa:**`, and CI calls the same `qa:` tasks.
  - Every check runs in exactly one CI job, named there or reached through an aggregate, such as a `qa:py` that depends on `qa:py:*`. A check no job reaches runs only locally, so adding one means giving it a job, or saying beside it why CI leaves it out.
- Jobs that run a check install their tools with `jdx/mise-action` and `cache: false`, so a tool tracked at `latest` runs at its current release rather than at the one a cache was first saved with.
- Each job that sets up mise installs only the tools its check calls, hk included where it runs hk, named in `install_args`, or none with `install: false`, so a tool added to `mise.toml` for local use costs no job anything.
  - The workflow sets `MISE_TASK_RUN_AUTO_INSTALL: "false"`, so `mise run` does not install a tool its job left out, and the check fails on the missing command.
    - The runner image's own copy of a command hides that failure: a job that runs cargo without `rust` among its tools runs the image's toolchain, not `mise.toml`'s.
- Every job that gates a pull request is a required check in the main ruleset. A Dependabot PR that auto-merges waits on those checks and nothing else.

## Secret scanning

Secrets are scanned in three layers, each covering what the one before it cannot:

- **GitHub**: secret scanning and push protection are on. For a personal account's repository they cover provider tokens only; generic patterns such as private keys and database connection strings need an organization with Secret Protection ([GitHub docs](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enabling-secret-scanning-for-generic-patterns)).
- **Local hooks**: `secrets:commit-scan` and `secrets:push-scan`, the shared secret-scan tasks dotfiles lends ([tasks/README.md](../../tasks/README.md)).
- **CI**: for commits that never went through those hooks, two jobs scan each pull request's commits and each push to `main`:
  - `gitleaks/gitleaks-action` with `GITLEAKS_VERSION: latest` and `GITLEAKS_ENABLE_COMMENTS: "false"`. Its upstream limits are named in a comment, not worked around: a PR scan covers only the first 30 commits ([gitleaks-action#187](https://github.com/gitleaks/gitleaks-action/issues/187)) and skips what a merge commit brings in ([#236](https://github.com/gitleaks/gitleaks-action/issues/236)).
  - `trufflesecurity/trufflehog` with `--results=verified,unknown --fail-on-scan-errors`: the upstream-recommended results, plus failing when a commit could not be read rather than passing having scanned nothing.

## Scheduled runs

- `ci.yml` also runs weekly. Every job then meets the current release of its tools even when no pull request comes in, and both secret scans cover the whole history with their current rules.
- The networked link check runs in a workflow of its own, weekly, and gates nothing; the offline link check is one of the checks above and gates every pull request. Both workflows fail on a problem, so that GitHub notifies whoever last edited the cron line.
- A repository whose Dependabot PRs auto-merge also runs a daily workflow that fails while an open Dependabot PR has a failed check, since GitHub notifies no one of the CI runs Dependabot triggers, and a scheduled run's failure reaches whoever last edited its cron line.

## Actions

- Every action is pinned to a full commit SHA with its version in a trailing comment, as [GitHub's guidance](https://docs.github.com/en/actions/reference/security/secure-use) recommends, and Dependabot's `github-actions` entry moves the pins.
- Workflows grant `contents: read` and nothing more unless a job needs it.

## Dependency updates

- Every Dependabot entry for an ecosystem with a lockfile sets `allow: - dependency-type: "all"`, since a version update otherwise [moves only what the manifest names](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference#allow--).
  - Which ecosystems Dependabot reaches a lockfile-only dependency in differs and changes, so the line goes on every such entry rather than on the ones known to reach them.
