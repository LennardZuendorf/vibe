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
naming each stale file and the count. Inside a full or partial checkout that leg always runs;
on a bare copy of `oracles/` the check says freshness was not checked.

Reviewed live repairs may be pinned in `tests/oracles/accepted-divergences.tsv`.
Each row contains the oracle-relative path, frozen SHA256, live SHA256, and a
reason, separated by tabs. Both hashes must match the current bytes. Unknown
paths, duplicates, malformed rows, missing sources, and stale rows fail, even
when the source now equals the oracle. This retains the frozen reference and
refuses any unreviewed source drift. The table labels merged-main differences
separately from the cursor and hook repairs.

`bash oracles/freeze.sh` preflights and then re-copies **every** listed
path from its live source, regenerates the manifest, runs the check, and prints
the new hash to pin in `tests/oracles/run.sh` (after review — the pin is what
makes a regenerated manifest over a changed oracle fail). The source-availability
preflight refuses before copying. Copy failures after that preflight are not
transactional; the tool does not promise rollback.

An accepted-divergence table is never rewritten or removed by the freezer. A
reviewed re-freeze must retire or update those rows and the external manifest
pin explicitly. Otherwise the post-copy check fails on the now-stale hash pairs.

The refusal is two-pass: it resolves and proves every listed source exists
**before** the first byte is copied. So a missing live source (or a path with no
live-source rule) aborts with the freeze byte-identical on disk — the tool says
"nothing written" only when that is true. A copy loop that aborts mid-way would
leave a half-frozen tree behind a message denying it.

## Generation log

- gen-1 — frozen from commit `4e52d07bf34c391f5b5f8f5183aae1b8fdc1113f`.
- gen-2 — **all halves** re-frozen from the Track 0 tree (the v0.4 sweep:
  `docs/spec` routes, Rust tooling, hook wiring, plus the flow-v2 skills-v2 and
  route sweep). Whole, not flow+js only: a partial freeze would have left 9
  stale files — 7 cosmetic spec-side scripts plus the `plan.md`/`tech.md` setup
  templates, which ship to users through `setup.sh` and still said `.spec/`
  where the tree now says `docs/spec`.
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
