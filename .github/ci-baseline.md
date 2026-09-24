# CI baseline

The CI every public repository of this owner follows, dotfiles and k-boat among them.
A repository departs from it only where a comment in its workflow says why.

## Checks

- CI runs the repository's local `mise run pre-commit` checks by calling the same `qa:` tasks, one job per task, and `pre-commit` depends on `qa:**`. What a check runs is then the repository's `mise.toml` to say, in one place.
- Jobs install their tools with `jdx/mise-action` and `cache: false`, so a tool tracked at `latest` runs at its current release rather than at the one a cache was first saved with.
- Every job that gates a pull request is a required check in the main ruleset. A Dependabot PR that auto-merges waits on those checks and nothing else.

## Secret scanning

Secrets are scanned in three layers, each covering what the one before it cannot:

- **GitHub**: secret scanning and push protection are on. For a personal account's repository they cover provider tokens only; generic patterns such as private keys and database connection strings need an organization with Secret Protection ([GitHub docs](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enabling-secret-scanning-for-generic-patterns)).
- **Local hooks**: `secrets:commit-scan` and `secrets:push-scan` from this repository's shared tasks ([tasks/README.md](../tasks/README.md)).
- **CI**: for commits that never went through those hooks, two jobs scan each pull request's commits and each push to `main`:
  - `gitleaks/gitleaks-action` with `GITLEAKS_VERSION: latest` and `GITLEAKS_ENABLE_COMMENTS: "false"`. Its upstream limits are named in a comment, not worked around: a PR scan covers only the first 30 commits ([gitleaks-action#187](https://github.com/gitleaks/gitleaks-action/issues/187)) and skips what a merge commit brings in ([#236](https://github.com/gitleaks/gitleaks-action/issues/236)).
  - `trufflesecurity/trufflehog` with `--results=verified,unknown --fail-on-scan-errors`: the upstream-recommended results, plus failing when a commit could not be read rather than passing having scanned nothing.

## Scheduled runs

- `ci.yml` also runs weekly. Every job then meets the current release of its tools even when no pull request comes in, and both secret scans cover the whole history with their current rules.
- The networked link check runs in a workflow of its own, weekly, and gates nothing; the offline link check is a `qa:` task and gates every pull request. Both workflows fail on a problem, so that GitHub notifies whoever last edited the cron line.

## Actions

- Every action is pinned to a full commit SHA with its version in a trailing comment, as [GitHub's guidance](https://docs.github.com/en/actions/reference/security/secure-use) recommends, and Dependabot's `github-actions` entry moves the pins.
- Workflows grant `contents: read` and nothing more unless a job needs it.
