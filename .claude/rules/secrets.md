---
description: Keep secrets out of .env and mise config, and supply them through fnox
paths:
  - "**/.env"
  - "**/.env.*"
  - "**/fnox*.toml"
  - "**/mise*.toml"
  - "**/.mise*.toml"
  - "**/mise/**/*.toml"
  - "**/.mise/**/*.toml"
---

# Secrets

- Keep secrets out of `.env`, `.env.*` and mise's `[env]`, which are then config like any other: read them freely, and report one that holds a secret to the user by file and variable name, never by value.
- A secret lives in the macOS keychain, named in the main checkout's `fnox.local.toml`; run the command that needs it as `fnox exec -- <command>`.
- When a command fails for want of a key, look for it there rather than in `.env`, and never write a key into `.env` or mise's `[env]`.
- A key not yet stored is the user's to add, with `mise run secrets:add <NAME>`.
