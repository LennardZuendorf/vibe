---
type: feature-plan
feature: spec-cli-migration
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Implementation Plan

Build the `uv` workspace skeleton first (all four package dirs, frozen
`pyproject` deps + lock, `vibe-core`, shared `conftest`), then fill packages
bottom-up: `vibe-core` → re-home `vibe-flow` → port `vibe-spec` (the real work,
easy→hard) → assemble `vibe`. Byte-parity suites vs the bash originals are the
merge gate; bash retires only under R4 after the preflight guarantee. The asset
source-of-truth cutover is its own late, atomic unit.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Three apps, split by consumer and import tier | spec-cli-migration/1, /2, /8 |
| R2 | Native parity for all six spec commands | spec-cli-migration/3, /4, /5, /6, /7 |
| R3 | One behavior source, two renderers | spec-cli-migration/2, /7, /8 |
| R4 | Skill context stays runnable; retirement gated | spec-cli-migration/9 |

---

## Units

### spec-cli-migration/1 — Workspace skeleton

**Goal:** `uv` workspace with four package dirs, each `pyproject` frozen (deps +
the three console_scripts), one `uv.lock`, shared root `conftest.py` (move
`bash_ref`, `home_sandbox`, `target_project`). `vibe-flow`/`vibe-spec`/`vibe`
`--help` all run; the import-cost test skeleton exists.
**Requirements:** R1
**Dependencies:** —
**Verification:** `uv sync` builds the workspace; all three `--help` exit 0; the
two agent apps import stdlib-only (fresh-interpreter `sys.modules` assertion).

### spec-cli-migration/2 — `vibe-core`

**Goal:** Port the shared stdlib primitives — `assets.py`, `markers.py` (the
tested pairing/reversal guard), `errors.py`, `paths.py` (root-find + atomic
write). No third-party imports.
**Requirements:** R1, R3
**Dependencies:** /1
**Verification:** `test_markers` (reused) green under `vibe_core`; `vibe-core`
imports with zero third-party deps.

### spec-cli-migration/3 — Re-home `vibe-flow` (+ subsume `vibe-hook`)

**Goal:** Move `machine`/`cursor`/`policy`/`orders`/`hook` into `vibe_flow`
unchanged; add the argparse `app.py` (`vibe-flow {hook,status,next,go,check,
orders}`) + plain flow-verb renderer; move the flow parity tests.
**Requirements:** R1
**Dependencies:** /2
**Verification:** `test_parity_policy`/`test_parity_orders` green under
`vibe_flow`; `vibe-flow hook guard` byte-matches the prior `vibe-hook guard`;
import-cost test green (no typer/rich).

### spec-cli-migration/4 — `vibe-spec`: list + setup

**Goal:** `vibe_spec.model` (frontmatter parse) + `listing.py` + `setup.py` +
plain renderer; argparse `vibe-spec list` / `vibe-spec setup`.
**Requirements:** R2
**Dependencies:** /2
**Verification:** parity tests byte-match `list-specs.sh` (root/features/empty)
and `setup.sh` (fresh/partial/full).

### spec-cli-migration/5 — `vibe-spec`: lessons-for

**Goal:** `lessons.py` block parse + markdown/inject/json; `vibe-spec lessons-for`.
**Requirements:** R2
**Dependencies:** /4 (reuses `model.py`)
**Verification:** parity byte-matches all three formats, multi-tag, no-match.

### spec-cli-migration/6 — `vibe-spec`: scan-merges + promote

**Goal:** `merges.py` shared marker scan; `vibe-spec scan-merges` (table/json/
plain, unclosed→nonzero) + `vibe-spec promote` (`--dry-run`/`--target`, atomic).
**Requirements:** R2
**Dependencies:** /4
**Verification:** parity byte-matches all formats, dry-run vs write, unclosed error.

### spec-cli-migration/7 — `vibe-spec`: validate (the hard unit)

**Goal:** Port SF3–SF16 check-by-check into `validate.py`; honor env knobs; SF4
stays a bash shell-out. `vibe-spec validate` plain output is the parity target.
**Requirements:** R2, R3
**Dependencies:** /4 (reuses `model.py`)
**Verification:** parity byte-matches `validate.sh` (stdout+stderr+exit) over the
full `tests/spec/run.sh` matrix + error/warn/clean trees.

### spec-cli-migration/8 — Assemble `vibe` (human app)

**Goal:** `vibe_cli` typer app: top-level `init/doctor/update/uninstall/plugins/
setup` (provisioning re-homed) + rich-rendered `status/next/go/check/orders` and
`spec` group importing `vibe_flow`/`vibe_spec` logic (R3); `settings.py` writes
`vibe-flow hook <event>`; `doctor` gains the D5 python/PATH preflight.
**Requirements:** R1, R3
**Dependencies:** /3, /7
**Verification:** `vibe --help` shows all groups; provisioning tests
(init/uninstall discriminating, update-preserves-cursor) green; a provisioned
target's hooks call `vibe-flow hook …`.

### spec-cli-migration/9 — Asset cutover + bash retirement (gated)

**Goal:** D6 source-of-truth move — packages vendor owned assets; root `spec/`/
`flow/` + `.agents/skills/*` symlinks repoint into packages; `test_assets_sync`
retargets in the same commit. Then, only if R4's preflight guarantee is met and
retirement approved: repoint `spec/SKILL.md` at `vibe-spec …` and retire the
`.sh`. Default: keep bash.
**Requirements:** R4
**Dependencies:** /8 + explicit approval
**Verification:** asset-sync green post-cutover with no bundled≠source window; if
retired, a fresh non-bash target eval passes or the divergence is accepted in
writing; else bash remains and the feature ends at /8.

---

## Order rationale

Bottom-up on the dep graph: skeleton (/1) freezes all shared surface so builders
never touch `pyproject`/lock; `vibe-core` (/2) unblocks both stdlib apps;
`vibe-flow` (/3) is a low-risk re-home of shipped code that proves the argparse
+ shared-renderer pattern before the spec port; the spec units (/4–/7) go
easy→hard so `validate` lands last against a trusted parity harness; `vibe` (/8)
assembles once its imports exist; the asset cutover + retirement (/9) is last,
atomic, and gated. Parity suites gate every port; bash never retires before green.

## Waves

- **Wave A:** /1
- **Wave B:** /2
- **Wave C (parallel, dep /2):** /3, /4
- **Wave D (parallel, dep /4):** /5, /6, /7
- **Wave E:** /8 (deps /3, /7)
- **Wave F:** /9 (dep /8 + approval)
