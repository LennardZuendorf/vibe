---
type: concept
updated: 2026-10-19
---

# Spec vs Implementation

F1 and F25 are delivered. The rest of the v0.4 arc — F2 through F24 — is spec-ahead-of-code. The tree is still v0.3 apart from F1 and F25: rows 12–14 (js-core, content-layer, inject-triggers) are delivered, and the old rows 15–20 (machine-teeth, plugin-runtime, spec-js, delegation-redirect, spec-delta, doc-truth) were never started before this rewrite superseded them. F1 delivered the spec-path fix: the scripts `validate.sh`, `list-specs.sh`, `lessons-for.sh`, and `scan-merges.sh` walk up from the cwd to the directory holding `.git`, and a relative `SPEC_DIR` now resolves against the root found. `promote.sh`, `setup.sh`, and `check-drift.sh` are unchanged.

The v0.5 arc (C1–C6) is likewise spec-ahead — planned, not started. It gates on F22 (installer/release surface), with C4 after F16 and C5 after F14.
