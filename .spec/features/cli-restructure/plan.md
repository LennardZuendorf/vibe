---
type: feature-plan
feature: cli-restructure
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-04
---

# Feature: CLI restructure — Implementation Plan

Build the `uv` workspace skeleton first, then fill packages bottom-up:
`vibe-core` → re-home `vibe-flow` → port `vibe-spec` (easy→hard) → assemble
`vibe` → distribution → hard cutoff. Byte-parity suites vs the bash originals are
the merge gate; golden fixtures are frozen before the `.sh` are removed at the
final cutover unit.

**Feature gate:** starts when `vibe-cli` is `BUILT`/`DONE` (root Feature Sequence
feature 9); this feature supersedes its flow-CLI + the flow/spec bash. Registered
as feature 10 (PLANNED) in the root [`.spec/plan.md`](../../plan.md) Feature Sequence.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Three apps, minimal deps, fast hot path | cli-restructure/1, /2, /3 |
| R2 | Native parity for all six spec commands | cli-restructure/4, /5, /6, /7 |
| R3 | One behavior source, two renderers | cli-restructure/3, /7, /8 |
| R4 | Hard cutoff to Python, guarded by preflight | cli-restructure/10 |
| R5 | Installable from GitHub; in-place update migrates | cli-restructure/8, /9 |

---

## Key Technical Decisions

D1–D9 are recorded in [product.md](product.md) § Decisions. Load-bearing for the
plan: bottom-up dep order (D1); flow verbs are a rewrite, not a move (tech §Impl);
`merges.py` is net-new, not guard reuse (D8); reproduce bash quirks for parity,
drop SF4 (D9); freeze golden fixtures before deleting bash (D6); one lockstep
version freezes inter-package deps in /1 (D7).

---

## Units

### cli-restructure/1 — Fully-loaded workspace skeleton

**Goal:** The **single sequential foundation** that lets every later wave fan out
parallel-safe: builders in /4–/7 then create only their own leaf module + own
test and touch **nothing shared** (governing rule — see § Parallel-safety
contract). /1 owns all shared surface:

- **Workspace + freeze.** `uv` workspace, four package dirs, all `__init__.py`
  (empty), each `pyproject` frozen (deps + the three console_scripts + one
  lockstep version), one `uv.lock`. `pyproject` is never edited again except /9's
  workspace step.
- **Both `app.py` wired and FROZEN.** `vibe_spec/app.py` declares **all six**
  subparsers (`list setup lessons-for scan-merges promote validate`), each
  dispatching by **direct import** from its leaf module (`from vibe_spec.validate
  import run_validate` — never an `__init__` re-export hub). Same for
  `vibe_flow/app.py`. No later unit edits either `app.py`.
- **Every leaf module stubbed** so the frozen `app.py` imports cleanly:
  `vibe_spec/{listing,setup,lessons,merges,validate}.py` each a placeholder
  handler raising `NotImplementedError`; each exposes its own `render_plain()`
  (no shared `render.py` in `vibe_spec` — see /7 note). `vibe_flow` leaf stubs
  as its verbs need.
- **`vibe_spec/model.py` implemented here** (frontmatter parse) — the one shared
  util the governing rule says the foundation must own; removes the /4→/5/6/7
  cross-wave read edge.
- **Generic, name-parametrized `conftest.py`** (`bash_ref` runs *any* named
  script; `golden` loads *any* named fixture; `home_sandbox`, `target_project`)
  + pre-created `tests/golden/<command>/` dirs — so no builder edits conftest.
- **Vendor every `_assets/` tree**: `vibe-flow/_assets/` (state-machine.json,
  state.example.json, skills/vibe/**, deps.json, adapters.json,
  templates/AGENTS.md) and `vibe-spec/_assets/` (skills/spec/**,
  reference/templates/**, validate fixtures) — recursive package-data glob in
  each `pyproject`.

**Requirements:** R1
**Dependencies:** —
**Files:** tech.md § Workspace layout; § Parallel-safety contract below.
**Test scenarios:** `uv sync` builds the workspace; `vibe`/`vibe-flow`/`vibe-spec`
`--help` exit 0 (frozen `app.py` imports all stubs cleanly); every `vibe-spec
<subcmd> --help` exits 0 while its body raises `NotImplementedError`;
fresh-interpreter `sys.modules` assertion — `vibe-flow hook` path imports
stdlib-only; `vibe-spec` whole-app import stays lean (no typer/rich/pydantic);
`vibe_spec/model.py` parses a known frontmatter fixture.
**Verification:** all of the above green.

### cli-restructure/2 — `vibe-core`

**Goal:** Shared stdlib primitives — `assets.py`, `markers.py` (the tested
pairing/reversal guard, used by CLI provisioning — **not** the spec merge scan,
D8), `errors.py`, `paths.py` (root-find + atomic write).
**Requirements:** R1
**Dependencies:** /1
**Files:** `vibe_core/*`.
**Test scenarios:** `test_markers` (reused) green under `vibe_core`; zero third-party imports.

### cli-restructure/3 — Re-home `vibe-flow` (oracles move, verbs rewrite)

**Goal:** Move `machine`/`cursor`/`policy`/`orders`/`hook` unchanged (+ their
parity tests). **Rewrite** the verbs: extract `status`/`next`/`go`/`check`/
`orders` logic (from `flow_cmds.py` and `spec_cmd.py`) into `verbs.py` returning
result objects (`TransitionResult` etc., R3), add argparse `app.py` + plain
`render.py`. Subsume `vibe-hook` → `vibe-flow hook`.
**Requirements:** R1, R3
**Dependencies:** /2
**Files:** `vibe_flow/*`; tech.md § Contract.
**Test scenarios:** `test_parity_policy`/`test_parity_orders` green under
`vibe_flow`; `vibe-flow hook guard` byte-matches prior `vibe-hook guard`; each
verb's logic function returns a result with no printing; hot-path import-cost test green.

### cli-restructure/4 — `vibe-spec`: list + setup

**Goal:** Fill the `listing.py` + `setup.py` stubs (bodies + each module's own
`render_plain`); `model.py` and the frozen `app.py` subparsers already exist from
/1. Wire `vibe-spec list` / `vibe-spec setup` via the pre-declared dispatch. Honor
`${SPEC_DIR:-.spec}` where the origin does (list/scan) and **not** where it
doesn't (setup); rewrite setup's trailing path/hint lines for the new layout
(excluded from parity, R2). **Touches only `listing.py` + `setup.py` + their
tests** — never `app.py`, `model.py`, `conftest`, or `pyproject`.
**Requirements:** R2
**Dependencies:** /2 (vibe-core; transitively /1's `model.py` + frozen `app.py`)
**Files:** `vibe_spec/{listing,setup}.py`, `tests/test_listing.py`, `tests/test_setup.py`.
**Test scenarios:** parity byte-matches `list-specs.sh` (root/features/empty, `LC_COLLATE`
order) and `setup.sh` (fresh/partial/full) minus the divergent hint lines; golden captured.

### cli-restructure/5 — `vibe-spec`: lessons-for

**Goal:** Fill the `lessons.py` stub — block parse + markdown/inject/json +
its own `render_plain`; `vibe-spec lessons-for` already wired in the frozen
`app.py`. Reproduce the **unescaped** hand-rolled JSON (`printf %s`) and
case-insensitive regex tag match. Reuses `model.py` from /1. **Touches only
`lessons.py` + its test.**
**Requirements:** R2
**Dependencies:** /2 (vibe-core; `model.py` from /1 — no /4 edge)
**Files:** `vibe_spec/lessons.py`, `tests/test_lessons.py`.
**Test scenarios:** parity byte-matches all three formats incl. a lesson body with a `"`
(proves the unescaped-JSON reproduction), multi-tag, no-match; golden captured.

### cli-restructure/6 — `vibe-spec`: scan-merges + promote

**Goal:** Fill `merges.py` — a **net-new** multi-block/nesting scanner over bare
`<!-- merge -->` (D8, not marker-guard reuse) + its own `render_plain`;
`vibe-spec scan-merges` (table/json/plain, unclosed→nonzero) + `vibe-spec
promote` (`--dry-run`/`--target`, `mktemp` in target dir, `printf '\n%s\n'`
framing, strict marker equality) already wired in the frozen `app.py`; promote's
atomic write uses `vibe_core/paths.py`. **Touches only `merges.py` + its test.**
**Requirements:** R2
**Dependencies:** /2 (vibe-core paths; no /4 edge)
**Files:** `vibe_spec/merges.py`, `tests/test_merges.py`.
**Test scenarios:** parity byte-matches all formats, dry-run vs write, target
byte-unchanged on nested/reversed/unclosed → exit 1; golden captured.

### cli-restructure/7 — `vibe-spec`: validate (the hard unit)

**Goal:** Port SF3–SF16 check-by-check into `validate.py` honoring the parity
boundary (tech §Impl): hardcoded `.spec`, regex-as-data (SF12 substring, SF14
regex, greedy links), byte prefixes, `wc -l`, glob collation. **SF4 dropped** (D9).
**Requirements:** R2, R3
**Dependencies:** /4 (reuses `model.py`)
**Files:** `vibe_spec/validate.py`, `render.py`.
**Test scenarios:** parity byte-matches `validate.sh` (stdout+stderr+exit) over the
full `tests/spec/run.sh` matrix + error/warn/clean trees, `VIBE_DESIGN_LINT` unset;
golden captured.

### cli-restructure/8 — Assemble `vibe` (human app) + install migration

**Goal:** `vibe_cli` typer app: top-level `init/doctor/update/uninstall/plugins/
setup` + rich-rendered verbs and `spec` group importing `vibe_flow`/`vibe_spec`
logic (R3). Provisioning: `settings.py` **migration** (unmerge legacy `vibe-hook
…`, then merge `vibe-flow hook …`); `doctor` updates the hook check + adds the
python/PATH preflight; `skills.py` resolves the two sibling packages' `_assets/`
via importlib; `init_cmd` binary/PATH set → the three tools.
**Requirements:** R3, R5
**Dependencies:** /3, /7
**Files:** `vibe_cli/*`, `provision/*`.
**Test scenarios:** `vibe --help` shows all groups; **discriminating** uninstall
(user file in each shared dir survives, shipped file gone) under the 2-package
layout; `update` preserves a live cursor; **migration test** — a settings.json with
legacy `vibe-hook` entries ends with exactly the three `vibe-flow hook` and zero
stale; provisioning locates sibling `_assets` under a real (non-workspace) install.

### cli-restructure/9 — Distribution (GitHub install script)

**Goal:** Rework `install.sh` into the GH bootstrap (D7): remove any prior combined
`vibe-flow` install, `uv tool install` the three tools from the checkout,
`vibe init`. One lockstep version. Reconcile ownership with the `install-tooling`
feature (supersede note).
**Requirements:** R5
**Dependencies:** /8
**Files:** `install.sh`, `cli/pyproject.toml` (workspace).
**Test scenarios:** from a **fresh checkout**, the script puts `vibe`/`vibe-flow`/
`vibe-spec` on `PATH` (workspace path-dep resolution proven) and `vibe init`
provisions; upgrading a prior combined install removes the old binaries first.

### cli-restructure/10 — Hard cutoff (both halves)

**Goal:** After all commands are parity-green and golden fixtures frozen under
`LC_ALL=C` (D6): make package `_assets/` canonical, **delete** root `spec/`/`flow/`
and repoint every `.agents/skills/*` symlink + `install.sh` copy + doc reference
through the installed package, retarget `test_assets_sync`; repoint `spec/SKILL.md` (incl. the
`!` embed → preflight-guarded) + `flow/SKILL.md` + every skill/`README`/strategy/
feature-file + the merged `templates/AGENTS.md` + `spec/agents/*/SKILL.md` at the
binaries; rewrite `ci.yml` (validate step → `vibe-spec validate`; drop `.sh`
shellcheck); port/retire `tests/spec/run.sh` (and `tests/run.sh` wiring) to pytest;
retire the plugin bash-shim hooks; **remove** flow *and* spec `.sh`; update
`README`/docs/`CHANGELOG` (three tools, `vibe-flow hook`, GH install).
**Requirements:** R4
**Dependencies:** /9 (all parity-green; golden frozen)
**Files:** all listed references; tech.md § Asset ownership.
**Test scenarios:** repo-wide grep finds no live `.sh` invocation across skills,
hooks, `tests/`, `.github/`, `install.sh`, the `AGENTS.md` template; no root
`spec/`/`flow/` tree remains and provisioning copies from the installed package;
asset-sync green with no bundled≠source window; a missing-binary `!` embed degrades
to a documented line; golden regression tests still pass with bash gone; a fresh
install-script target runs `vibe-spec`/`vibe-flow`; stranger eval passes on a
Python-present sandbox.

---

## Order rationale

Bottom-up on the dep graph: skeleton (/1) freezes shared surface + version;
`vibe-core` (/2) unblocks both stdlib apps; `vibe-flow` (/3) re-homes shipped
oracles and rewrites the verbs, proving the argparse + result-object pattern
before the spec port; the spec units (/4–/7) go easy→hard with the parity boundary
pinned so `validate` lands last against a trusted harness; `vibe` (/8) assembles +
migrates existing installs; distribution (/9) makes the tools reachable (R4's
preflight depends on it); the hard cutoff (/10) is last and atomic — assets
canonical, all bash removed, every reference repointed — only after golden fixtures
are frozen. Bash never leaves before its Python replacement is proven and a
reference is captured.

## Waves

- **Wave A:** /1
- **Wave B:** /2
- **Wave C (parallel, dep /2):** /3, /4
- **Wave D (parallel, dep /4):** /5, /6, /7
- **Wave E:** /8 (deps /3, /7)
- **Wave F:** /9 (dep /8)
- **Wave G:** /10 (dep /9)
