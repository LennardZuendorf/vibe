---
type: tech
applies:
  - "oracles/**"
  - "tests/parity/**"
  - "crates/vibe-parity/**"
  - "crates/vibe-core/**"
updated: 2026-10-19
---

# Oracles and parity

`oracles/js/` freezes `flow/engine/*.mjs` verbatim; `oracles/bash/` freezes
every current script plus both fallbacks; `oracles/MANIFEST.sha256` is
checked in CI so an oracle can only change by an explicit, reviewed commit.
JS bugs found during parity work become pinned `expected-divergence` cases,
fixed in Rust only — the oracle never changes to match a bug.

`crates/vibe-parity` runs the oracle and the candidate over a **generated**
corpus (`tests/parity/corpus/`) and diffs exit code, stdout bytes, and
every resulting file's bytes (cursor, AGENTS.md, `settings.json`). Corpus
classes derive from `state-machine.json` and `policy.json`: cursors ×
{feature, none, malformed, missing}, policies × one mutation per rule,
tool inputs × path class × state. An empty corpus class fails the run — a
floor is asserted, not assumed.

`vibe_core::glob::glob_match` is path-aware (`*` stops at `/`) for spec
`applies:` globs. The flow write policy uses fnmatch without
`FNM_PATHNAME` (`*` crosses `/`); its port (F5) needs its own matcher,
proven against `oracles/bash/flow/scripts/detect-context.sh` and
`oracles/js/policy.mjs`.
