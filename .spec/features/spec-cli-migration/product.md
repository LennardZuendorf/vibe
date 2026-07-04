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
  Chosen because it is the proven pattern in this repo (vibe-cli) and it makes
  R2 unconditional — the skill context never loses its bash path.
- **D2 — `vibe spec` groups all six.** `validate` and `setup` move from
  subprocess wrappers to native; `list`, `lessons-for`, `promote`,
  `scan-merges` are added. One `spec` Typer group, one `register(app)` contract.
- **D3 — Shared logic is importable and rich-path only.** The spec commands are
  human/agent-invoked, never on the per-Edit guard hot path, so they may use
  `rich`/`pydantic`; but the shared logic stays in importable modules so tests
  and future callers don't go through Typer. (No second entry point is needed —
  this is not a hot path, unlike `vibe-hook`.)

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

1. **Reversing vibe-cli R5 / D1.** Root [vibe-cli](../vibe-cli/product.md)
   explicitly decided the spec half stays standalone bash and is *not*
   rewritten. This feature proposes a native Python path. Confirm the intent is
   to add a parallel path (safe, R2-preserving) and treat full retirement as a
   later, separately-approved decision — **not** to make Python a hard
   dependency of the spec skill now.
2. **Is `vibe` ever guaranteed in a skill context?** If yes (e.g. a future
   install invariant), retirement (R4) becomes viable. If no, bash must stay.
   This is the single fact that decides the end-state.
3. **`validate.sh` fidelity.** It is ~640 lines of `awk`/`grep`/`sed` with many
   heuristic checks (SF3–SF16). Porting it to Python is the largest and riskiest
   unit; confirm byte-parity (not just "equivalent") is the bar.
