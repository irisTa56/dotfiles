---
description: Keep secrets out of .env and mise config, and supply them through fnox
paths:
  - "**/.env"
  - "**/.env.*"
  - "**/fnox*.toml"
  - "**/mise*.toml"
  - "**/.mise*.toml"
  - "**/.config/mise/config*.toml"
---

# Secrets

- `.env`, `.env.*` and mise's `[env]` hold no secret on this machine, so read them as you would any config; one that does is a slip to report to the user by file and variable name, never by value.
- A secret lives in the macOS keychain, and a repository that needs one names it in its own `fnox.local.toml`; run the command that needs it as `fnox exec -- <command>`.
- When a command fails for want of a key, look for it there rather than in `.env`. Never write a key into `.env` or mise's `[env]`; a key not yet stored is the user's to add, with `fnox set <NAME> --provider keychain` run from the repository root.
