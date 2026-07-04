---
type: feature-product
feature: spec-cli-migration
sibling: tech.md
parent: ../../product.md
updated: 2026-07-04
---

# Feature: spec-CLI migration — Product

Fold the six spec-framework shell scripts (`validate`, `setup`, `list-specs`,
`lessons-for`, `promote`, `scan-merges`) into native `vibe spec` subcommands, the
same way [vibe-cli](../vibe-cli/product.md) folded the flow scripts into `vibe`.
Today `spec_cmd.py` only *wraps* two of the six by shelling out; the other four
have no CLI surface at all, and every one still lives as bash that the `spec`
skill invokes by path. This feature makes the spec commands first-class,
Python-native, and consistent with the rest of the CLI — **without dropping the
bash path the skill context depends on**.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | Native Python implementations of the six spec scripts behind `vibe spec {validate,setup,list,lessons-for,promote,scan-merges}`; the byte-parity suite that pins each against its bash origin; the shared `spec/` logic module(s) under `src/vibe/`; the decision record on whether/when the `.sh` originals retire |
| **Does not own** | The `.spec/` document *format* and validation *rules* (owned by the `spec` skill — this feature reproduces behavior, it does not change it); `state-machine.json` / cursor; the flow commands already ported by vibe-cli; the `spec` skill's SKILL.md routing prose (touched only if/when the skill is repointed at the CLI — a gated decision, see R4) |

**Supersedes (conditionally).** If R2's decision goes to "retire bash," this
feature supersedes ownership of `spec/scripts/*.sh` from the `spec` skill bundle
into the CLI, exactly as vibe-cli superseded the flow scripts. Until that
decision is made, the bash scripts remain canonical and this feature adds a
parallel Python path only.

---

## Requirements

### Requirement: Native parity for all six spec commands (R1)

The CLI SHALL expose every spec script as a native `vibe spec` subcommand whose
output and exit code are byte-identical to the current bash script across a
fixture matrix, reimplementing the logic in Python rather than shelling out.

#### Scenario: Native validate matches the bash script byte-for-byte

- **Given** a `.spec/` tree with a known mix of errors and warnings
- **When** `vibe spec validate` runs (native) and `bash spec/scripts/validate.sh` runs
- **Then** their stdout, stderr, and exit codes are identical, and no `bash` process is spawned by the CLI path

#### Scenario: The four unwrapped scripts gain CLI surface

- **Given** the current CLI, which wraps only `validate` and `setup`
- **When** the feature lands
- **Then** `vibe spec list`, `vibe spec lessons-for <tag>`, `vibe spec promote <feature>`, and `vibe spec scan-merges` exist with the same flags (`--format`, `--dry-run`, `--target`) and outputs as their scripts

### Requirement: Skill context stays runnable throughout (R2)

The migration MUST NOT break any context in which the `spec` skill runs today.
A skill context is not guaranteed to have Python or `vibe` on `PATH`; the spec
half is designed as the standalone, any-agent, zero-runtime half (root
[vibe-cli R5](../vibe-cli/product.md), decision D1). Every step SHALL leave a
bash-runnable path in place until an explicit decision retires it.

#### Scenario: Spec skill works with no Python installed

- **Given** a target with the `spec` skill vendored but no `vibe` on `PATH`
- **When** an agent follows `spec/SKILL.md` (e.g. runs `validate.sh`, embeds `list-specs.sh`)
- **Then** every referenced command still runs via bash, unchanged, and the skill behaves exactly as before

#### Scenario: Asset-sync invariant preserved

- **Given** `test_assets_sync.py`, which pins each bundled `_assets/skills/spec/scripts/*.sh` byte-identical to its `spec/` source
- **When** the native port lands
- **Then** the sync test still passes (the bash sources are unchanged, or the retirement decision updates the invariant deliberately, never silently)

### Requirement: Parity suite is the merge gate (R3)

Each ported command SHALL ship a byte-for-byte parity test against its bash
origin, skipped-with-message when `bash` is absent, and the bash original SHALL
NOT be retired until its parity test is green — mirroring the vibe-cli parity
gates.

#### Scenario: Divergence blocks the merge

- **Given** a native command whose output differs from its bash origin on any fixture
- **When** the parity suite runs in CI
- **Then** the suite fails and the command is not considered migrated

### Requirement: Retirement is a deliberate, separate decision (R4)

Whether to repoint `spec/SKILL.md` at `vibe spec ...` and retire the `.sh`
originals SHALL be an explicit decision with its own gate, not a side effect of
adding the native path. The default end-state after this feature is a parallel
Python path with bash retained; retirement happens only if the skill-context
runtime guarantee (R2) can be met.

#### Scenario: Bash retained by default

- **Given** the native commands are landed and parity-green
- **When** no explicit retirement decision has been approved
- **Then** the `.sh` scripts and the skill's bash invocations remain in place and canonical

---

## Decisions

- **D1 — Mirror the flow migration exactly.** Dual implementation: native
  Python behind `vibe spec`, bash retained, byte-parity suite as the merge gate.
  Chosen because it is the proven pattern in this repo (vibe-cli); bash stays as
  the parity oracle and skill-context fallback until R4's preflight guarantee.
- **D2 — `vibe spec` group; flow + management stay top-level.** `validate` and
  `setup` move from subprocess wrappers to native; `list`, `lessons-for`,
  `promote`, `scan-merges` are added under one `spec` Typer group via the
  existing `register(app)` contract. Flow verbs (`status`/`next`/`go`/`check`/
  `orders`) and management (`init`/`doctor`/`update`/`uninstall`/`plugins`) stay
  top-level to avoid a breaking rename of the shipped CLI; the three-domain model
  surfaces via `vibe --help` sectioning + the `spec`/`vibe` skill routers. A
  strict `vibe flow …` group is a deferred, deliberately-breaking option.
- **D3 — Spec *logic* is stdlib-only and importable; rich is the human layer.**
  `vibe/spec/*.py` is written import-stdlib (no `typer`/`rich`/`pydantic`), so it
  runs on the minimal install and is reusable by tests, the hook path, and a
  future skill-repoint. Only the `vibe spec` command *rendering* uses `rich`.
  Extends the standing hot-path rule to the spec half. No second entry point —
  spec is not a per-Edit hot path.
- **D4 — One package, layered by dependency (not split by domain).** vibe's dep
  tree is tiny (pure-Python `typer`/`rich`, `pydantic` trimmed or made an
  extra), so a GSD-style multi-package/multi-binary split is rejected — it adds
  release/versioning/PATH overhead for negligible footprint gain. "Reduce
  necessary install" is met by import-cost layering: minimal = `vibe-hook` +
  stdlib logic (zero third-party deps); full = `vibe` with the rich extra. Two
  console_scripts total, unchanged from today.
- **D5 — Python-preflight replaces the bash-standalone guarantee.** Python is an
  accepted dependency (`brew`/`uv` provide it). `vibe doctor` gains a
  Python-and-`vibe`-on-`PATH` check; that preflight — not "bash forever" — is
  what makes the skill context safe to call `vibe spec` and what unlocks R4
  retirement.

---

## Non-Goals

- Changing any validation rule, output wording, or `.spec/` format — this is a
  behavior-preserving port, not a redesign.
- Rewriting `spec/SKILL.md` routing to call `vibe` (gated by R4).
- Deleting the `.sh` scripts in this feature (gated by R4).
- Porting the spec skill's *subagents* (`spec-tracer`, etc.) — they are prompt
  assets, not scripts.

---

## Open Questions

1. **~~Reversing vibe-cli R5 / D1.~~ RESOLVED (2026-07-04).** Python-as-a-
   dependency is accepted (`brew`/`uv` provide it). Intent: add the native path
   in parallel now (D1); make the skill context safe via a `vibe doctor` Python/
   `PATH` preflight (D5); retire bash later under R4 once that preflight is
   wired. Not a silent refactor — the reversal is a recorded decision.
2. **~~Is `vibe` guaranteed in a skill context?~~ RESOLVED (2026-07-04).** Not
   intrinsically, but the D5 preflight (`vibe doctor` / a `vibe check`-style
   step) establishes the guarantee at setup time, which is what unlocks R4.
3. **`validate.sh` fidelity (OPEN).** ~640 lines of `awk`/`grep`/`sed`
   (SF3–SF16). Porting to Python is the largest, riskiest unit; byte-parity (not
   "equivalent") is the bar. The optional SF4 network lint stays a bash shell-out.
4. **Machine-output fast path (OPEN).** Whether to expose the agent/CI-consumed
   outputs (`list`/`lessons-for`/`scan-merges --format json`, `validate`
   exit-code) through the stdlib entry point for zero-`rich` startup, or keep
   everything under the one rich `vibe` command. See tech.md § Packaging.
