---
type: tech
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-10-19
---

# Migrate

`vibe-spec migrate --to docs/spec [--dry-run]`: `git mv` the tree; write
`root` into `.vibe/spec.json`; rewrite `.spec/` literals across an <!-- literal -->
explicit file set (the new root, `.vibe/*.json`, `.claude/**`,
`.agents/**`, `tests/**`, root `*.md`, template files), tagging exempted
lines `<!-- literal -->`; then a **population assertion** — literals
before ≥1, literals after ==0, files scanned ≥ a floor — or the migration
reverts and aborts rather than leaving a half-migrated tree.
`oracles/` and `tests/parity/corpus/` hold `.spec/` literals by design <!-- literal -->
(frozen oracles and legacy-layout fixtures) and are excluded from the
rewrite and from the population count. Inserts
`<!-- vibe-spec: allow SF-nn -->` markers for any newly-warn-eligible
check so nothing appears that the owner cannot act on today. This repo
migrates in one commit, late in the sequence (plan.md, F20).

`--layout layered` runs after the move: `{tech,product,plan}-<topic>.md`
→ `<kind>/<topic>.md`, each D-record → `decisions/dNN-<slug>.md`, hubs
seeded from templates. Splitting oversized prose into kind notes is
agent-led via the spec skill's restructure mode; the gate is
`validate --strict` green.
