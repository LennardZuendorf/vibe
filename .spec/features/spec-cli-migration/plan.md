---
type: feature-plan
feature: spec-cli-migration
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Implementation Plan

Port the six spec scripts to native Python behind `vibe spec`, easiest-first, so
the parity harness and shared modules are proven on low-risk commands before
`validate.sh` (the hard one). Bash stays canonical until R4's retirement gate.
Parity suites are the merge gate — a command is "migrated" only when its
byte-for-byte test against the `.sh` origin is green.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Native parity for all six spec commands | spec-cli-migration/1, /2, /3, /4, /5, /6 |
| R2 | Skill context stays runnable throughout | spec-cli-migration/0, /7 |
| R3 | Parity suite is the merge gate | spec-cli-migration/1, /2, /3, /4, /5, /6 |
| R4 | Retirement is a deliberate, separate decision | spec-cli-migration/7 |

---

## Units

### spec-cli-migration/0 — Scaffold + dep layering + preflight

**Goal:** Create the stdlib-only `vibe/spec/` package (no `typer`/`rich`/
`pydantic` imports) + the parity fixture wiring (extend the `bash_ref` conftest
pattern to `spec/scripts/*.sh`); apply D4 dependency layering (trim/extra
`pydantic`, confirm `vibe-hook` still imports stdlib-only via the existing import-
cost test); add the D5 Python/`PATH` preflight check to `vibe doctor`. No spec
behavior yet. (R5-reversal decision is recorded — product.md OQ1/OQ2 resolved.)
**Requirements:** R2, R4
**Dependencies:** —
**Verification:** empty `vibe/spec/` imports with zero third-party deps; import-
cost test still green; `vibe doctor` reports the Python/`vibe`-on-`PATH` check; a
skipped-without-bash parity fixture resolves both source and bundled script paths.

### spec-cli-migration/1 — Port `list-specs`

**Goal:** Native `vibe spec list`; frontmatter/area parse in `spec/listing.py`.
**Requirements:** R1, R3
**Dependencies:** /0
**Verification:** `test_spec_listing_parity.py` byte-matches `list-specs.sh` over
root-only, features-present, and empty-`.spec` fixtures.

### spec-cli-migration/2 — Port `setup`

**Goal:** Native `vibe spec setup` (replace the subprocess wrapper); template
copy + lessons stub + writing-order stdout.
**Requirements:** R1, R3
**Dependencies:** /0
**Verification:** `test_spec_setup_parity.py` byte-matches `setup.sh` for
fresh, partial, and fully-existing `.spec/`; existing `test_spec_cmd.py` setup
test still passes.

### spec-cli-migration/3 — Port `lessons-for`

**Goal:** Native `vibe spec lessons-for <tag>`; block parse + markdown/inject/json.
**Requirements:** R1, R3
**Dependencies:** /0
**Verification:** `test_spec_lessons_parity.py` byte-matches all three formats,
multi-tag, and no-match (exit 0, empty) cases.

### spec-cli-migration/4 — Port `scan-merges` + `promote`

**Goal:** Native `vibe spec scan-merges` and `vibe spec promote`; shared marker
scan in `spec/merges.py`; `--format`, `--dry-run`, `--target`, unclosed-block
nonzero exit.
**Requirements:** R1, R3
**Dependencies:** /0
**Verification:** `test_spec_merges_parity.py` byte-matches table/json/plain,
dry-run vs write (atomic append), and the unclosed-block error path.

### spec-cli-migration/5 — Port `validate` (the hard unit)

**Goal:** Native `vibe spec validate` (replace the subprocess wrapper); port
SF3–SF16 check-by-check into `spec/validate.py`, honoring `VIBE_DESIGN_LINT`,
`SPEC_DIR`, `SPEC_ROOT_MAX_LINES`. SF4 network lint stays a bash shell-out or is
documented as a divergence (tech.md OQ2).
**Requirements:** R1, R3
**Dependencies:** /0, and /1 (reuses `spec/model.py` frontmatter parsing)
**Verification:** `test_spec_validate_parity.py` byte-matches `validate.sh`
(stdout + stderr + exit) over the full `tests/spec/run.sh` fixture matrix plus
error/warning/clean trees; `test_spec_cmd.py` validate parity test still passes.

### spec-cli-migration/6 — Wire the group + assets-sync check

**Goal:** All six subcommands mounted via `spec_cmd.register`; `--help` clean;
confirm `test_assets_sync.py` still green (bash sources untouched).
**Requirements:** R1, R3
**Dependencies:** /1, /2, /3, /4, /5
**Verification:** `vibe spec --help` lists six commands; full `cli/tests` green;
asset-sync unchanged.

### spec-cli-migration/7 — Retirement decision (deferred, gated)

**Goal:** Only if R4's guarantee (vibe on PATH in every spec-skill context) is
met: repoint `spec/SKILL.md` at `vibe spec ...`, retire the `.sh` originals, and
update the asset-sync invariant deliberately. Default: **do not** — keep bash.
**Requirements:** R2, R4
**Dependencies:** /6 + explicit approval
**Verification:** if executed, skill-context eval on a fresh non-Python target
still passes or the divergence is accepted in writing; if not executed, bash
remains and the feature ends at /6.

---

## Order rationale

Easiest-first (/1–/4) proves the shared `spec/model.py` parsing and the parity
harness on low-risk commands, so `validate` (/5) — the ~640-line `awk` port that
carries the whole risk — lands last against a trusted harness. /0 front-loads the
R5-reversal decision because everything downstream assumes it; /7 is deferred and
gated, never a side effect of adding the native path.
