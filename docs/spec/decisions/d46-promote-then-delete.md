---
type: decision
updated: 2026-10-19
---

- **D46 — Promote, then delete.** At compound, durable facts from a feature's superpowers artifacts are promoted into the layered spec and lessons; `vibe-spec promote <feature> --done` then deletes the artifacts and refuses if `docs/spec/` is unchanged since the branch base; `check-drift --pre-merge` fails while any `docs/superpowers/**` file remains.
