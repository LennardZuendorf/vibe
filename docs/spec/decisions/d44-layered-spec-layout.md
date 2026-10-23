---
type: decision
updated: 2026-10-19
---

- **D44 — Layered spec layout.** `docs/spec/` = root hubs (`product`, `tech`, `design`, `plan`; `lessons.md` kept) + kind folders created on demand (`areas/`, `flows/`, `concepts/`, `platform/`, `tech/`, `decisions/`, `questions/`, optional `parts/<part>/`). Lowercase-kebab unique names, relative markdown links, typed frontmatter (`code:` on areas, `applies:` on tech), hub ≤80 lines, note ≤60; over budget splits, never trims.
