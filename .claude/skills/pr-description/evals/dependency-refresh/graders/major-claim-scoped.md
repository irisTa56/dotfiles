---
type: llm
---

Facts of the change: react-router moved 6 → 7, a major update that reaches the application. `@types/node` also moved 20 → 22, a major update, but only in the build tooling, pulled in transitively. Every other direct dependency moved by a minor or patch version.

PASS if every statement in the body about which updates are major, or about the rest being minor or patch, is true given those facts. A statement scoped to the application or to direct dependencies (for example "react-router is the only major update reaching the app") is true. A body that makes no such statement also passes.

FAIL if the body makes such a statement that the `@types/node` bump contradicts, for example "react-router is the only major update" or "everything else is minor or patch" without a scope that excludes the build tooling or transitive dependencies.
