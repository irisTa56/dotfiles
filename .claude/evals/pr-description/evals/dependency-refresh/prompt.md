---
description: A dependency refresh with one hand-made exception, one pin left alone, and test metrics on offer
max_turns: 8
allowed_tools: [Skill]
---

Write the pull request description for this change. The repository has no PR template, and its merged PRs are written in English. Reply with the PR body only.

What I did:

- Ran `pnpm update --latest` in the web app, then regenerated `pnpm-lock.yaml`. That moved these direct dependencies: vite 6.2.1 → 6.4.0, vitest 3.0.4 → 3.2.1, zod 3.24.1 → 3.25.3, @tanstack/react-query 5.66.0 → 5.71.2, react-router 6.28.2 → 7.4.0, date-fns 4.1.0 → 4.1.2, clsx 2.1.0 → 2.1.1, typescript 5.7.3 → 5.8.2.
- react-router 7 removed `json()` and `defer()` from loaders, so I rewrote the four loaders under `src/routes/` to return plain objects. This is the only hand-written code change; everything else is the tool's output.
- I left eslint pinned at `<9` as it was, because our shared config hasn't moved to flat config yet.
- The lockfile refresh also bumped `@types/node` from 20 to 22 through vitest, but only the build tooling depends on it; the app bundle doesn't.
- `pnpm test` passed: 412 tests in 38.2 s, coverage 87.3 %. `pnpm build` passed and the bundle shrank from 214 kB to 209 kB.
