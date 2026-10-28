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

## Freshness

An oracle that has fallen behind its source is still self-consistent, so an
integrity check alone passes over a stale freeze. `check.sh` therefore also
compares every listed oracle with the live source it mirrors — `bash/<path>` is
`<path>` at the repo root, `js/<path>` is `flow/engine/<path>` — and fails
naming each stale file and the count. Inside a checkout that leg always runs;
on a bare copy of `oracles/` the check says freshness was not checked.

A freeze is all-or-nothing: `bash oracles/freeze.sh` re-copies **every** listed
path from its live source, regenerates the manifest, runs the check, and prints
the new hash to pin in `tests/oracles/run.sh` (after review — the pin is what
makes a regenerated manifest over a changed oracle fail). A half-freeze is not
expressible there, which is the point: re-copying one half while leaving the
other behind is exactly how the generation-1 record survived into generation 2.

## Generation log

- gen-1 — frozen from commit `4e52d07bf34c391f5b5f8f5183aae1b8fdc1113f`.
- gen-2 — **all halves** re-frozen from commit `06348b2` (the v0.4 sweep:
  `docs/spec` routes, Rust tooling, hook wiring). Whole, not flow+js only: a
  partial freeze would have left 9 stale files — 7 cosmetic spec-side scripts
  plus the `plan.md`/`tech.md` setup templates, which ship to users through
  `setup.sh` and still said `.spec/` where the tree now says `docs/spec`.
- gen-3 — **all halves** re-frozen on the flow-v2 branch, after Task 0.2 (route
  sweep) changed three live flow scripts that gen-2 had already frozen:
  `detect-context.sh` gained the `.planning/*` block rule (block in every state,
  `idle` the escape hatch) and `orders.sh`/`regen-active-rules.sh` had their
  D12/spec-root comments retruthed by Task 0.1. The behaviour change is the
  `.planning` arm; the other two are comments only. gen-2 is the last freeze
  that predates skills v2.
- gen-4 — **all halves** re-frozen on the flow-v2 branch for F6's shim (Task
  1.2). Two live sources moved: `.claude/hooks/pre-tool-use-guard.sh` and
  `.claude/hooks/stop-gate.sh` stopped being node-first and became adapters that
  exec `flow/bin/hook.sh <event>`. No parity subject moved: the hook legs compare
  against `flow/hooks-fallback/` (`bash/flow/hooks-fallback/**`), which is
  byte-identical to gen-3, and the frozen `.claude/hooks/` copies are not spawn
  targets in `tests/parity/*.json`. The only behaviour these two copies record is
  "the adapter forwarded to the shim", which is exactly what was shipped.
