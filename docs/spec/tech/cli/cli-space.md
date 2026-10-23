---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# Global spec space and blocks

`vibe space` — the personal layer above per-repo memory:

- **Global lessons library** at `~/.config/vibe/spec/lessons.md`. `vibe-spec
  lessons-for` gains an additive global source: answers merge repo lessons
  with global lessons; on tag conflict the repo entry wins (more specific
  memory beats general doctrine). Flow's orders and session start read the
  merged answer through the existing payload/budget machinery — no new
  channel, no new budget.
- `vibe space lessons edit` — open the global library. `vibe space lessons
  promote <tag>` — copy a repo lesson's rule into the global library
  (verbatim rule + tags; the repo-specific pattern prose stays behind).
- **Cross-repo visibility** — `vibe space list [--json]` shows each
  registered repo: path, flow/phase, feature, last ledger event, spec drift
  flag. `vibe space status <repo>` for one. Reads cursor, ledger, and spec
  JSON answers only — never another tool's internals.

## Global blocks

`vibe space blocks` — list/edit the instruct global tier
(`~/.config/vibe/instruct/`) and pin a personal git source for it. Sync is
`vibe-instruct sync` over the registered source (D32 unchanged); `vibe space`
only registers and edits — it never grows a second sync engine, asserted by
a `cargo metadata` test that `crates/vibe` pulls no network or fetch crate.
