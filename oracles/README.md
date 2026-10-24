# Frozen oracles

Byte-exact copies of the implementation the Rust port must match (D30, F3):

- `bash/` — every tracked script under `spec/scripts/`, `flow/scripts/`,
  `flow/hooks-fallback/` and `.claude/hooks/`, at its repo path, plus the
  four setup templates `spec/scripts/setup.sh` reads.
- `js/` — every non-test module of `flow/engine/`.

`MANIFEST.sha256` lists every file; `bash oracles/check.sh` fails on any
byte change, missing or unlisted file, or fewer files than its floor. An
oracle never changes to match a bug: a defect found during parity becomes an
`expected-divergence` pin in the parity suite, fixed in Rust only.

Frozen from commit 4e52d07bf34c391f5b5f8f5183aae1b8fdc1113f (generation 1).

Generation 2 (2026-10-24, plan `docs/superpowers/plans/2026-10-24-flow-v2.md`
Task 0.3): the flow half (`bash/flow/**`, `bash/.claude/hooks/**`) and the js
half (`js/**`) were re-frozen at their post-sweep live bytes — the gen-1
copies predated the docs/spec route-in, the superpowers-first feature layer,
and the GSD `.planning/` block, and would have ported a guard blind to
`docs/spec` writes. The spec half (`bash/spec/**`) stays at generation 1 as
vibe-spec's port record. From generation 2 on, the original rule applies
unchanged: an oracle never changes to match a bug — a defect found during
parity becomes an `expected-divergence` pin, fixed in Rust only.
