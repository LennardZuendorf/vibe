---
type: feature-product
feature: vibe-skill-consolidation
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---

# Feature: vibe Skill Consolidation — Product

Consolidate seven separate `vibe-*` skill directories into one `vibe/` skill directory following the agentskills.io progressive-disclosure model. Behavioural separation (per-state `caveman`, `writes`, `delegates`, `next`, `exit`) is preserved unchanged in `state-machine.json`; only the filesystem layout and the `skill` link field change.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)
**Related:** [../vibe-flow/product.md](../vibe-flow/product.md)

---

## Background

Root `plan.md` OPEN-2 (resolved as `vibe-flow/2`) chose to keep seven separate `vibe-*` shims on the grounds of distinct write surfaces and `caveman` levels per phase. That protection holds: per-state fields are in `state-machine.json`, not in the folder layout. This feature supersedes OPEN-2 on folder structure only — all per-state behaviour is unchanged.

The agentskills.io specification defines one skill as one directory. The `spec` skill already uses this correctly: lean `SKILL.md` entry + `feature.md`/`strategy.md` phase files loaded on demand. Seven `vibe-*` directories is folder proliferation, not behavioural necessity.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Seven `vibe-*` skill dirs SHALL be replaced by one `vibe/` skill dir. |
| R2 | `vibe/SKILL.md` SHALL be the lean entry point: user-invocable, routing table, all `<!-- vibe:orders:<state> -->` blocks, under 200 lines. |
| R3 | Each prior skill's procedural content SHALL live in a named phase file alongside `SKILL.md` (`setup.md`, `strategy.md`, `feature.md`, `quick.md`, `verify.md`, `compound.md`, `amend.md`). |
| R4 | Per-state behaviour — `caveman`, `writes`, `delegates`, `next`, `exit` — SHALL remain in `state-machine.json` unchanged. |
| R5 | `orders.sh` SHALL require no behavioural change; it resolves `$AGENTS_DIR/skills/$SKILL/SKILL.md` and `$SKILL` becomes `"vibe"`. |
| R6 | `state-machine.json` `skill` fields SHALL all be updated to `"vibe"`. |
| R7 | `install.sh` hardcoded `vibe-setup` paths SHALL be updated to `vibe` paths. |
| R8 | Cross-references to `vibe-{name}` in skill bodies and prose SHALL be updated to reference `vibe/` phase files or the consolidated skill name. |
| R9 | `tests/flow/run.sh` SHALL pass without modification (orders resolution is behaviour-identical). |

---

## Scope

| Owns | Does not own |
|---|---|
| `.agents/skills/vibe/` (new consolidated skill dir) | Per-state fields in `state-machine.json` (`writes`, `caveman`, `delegates`, `next`, `exit`) |
| Removal of `.agents/skills/vibe-{amend,compound,feature,quick,setup,strategy,verify}/` | `orders.sh` behaviour |
| `state-machine.json` `skill` field updates | `.spec/features/vibe-flow/product.md` R1–R11 |
| `install.sh` path updates | Root `plan.md` (OPEN-2 supersession intent recorded here; promote at compound) |
| `AGENTS.md` prose and routing table updates | Audit holes from separate audit run |
| Inter-skill cross-reference updates in skill bodies | |
| Spec file-layout blocks in `tech.md` + `vibe-flow/tech.md` | |

---

## GWT Scenarios

**orders.sh backward-compatibility**
- GIVEN `orders.sh` is called for `feature.impl`
- WHEN `state-machine.json` carries `"skill": "vibe"` and `vibe/SKILL.md` contains `<!-- vibe:orders:feature.impl -->`
- THEN output is byte-identical to the pre-consolidation output **except the leading
  `skill=` token** — deliberately changed from the old per-skill name (e.g.
  `skill=vibe-feature`) to `skill=vibe`, since the per-skill shims no longer exist (R8).
  (The `setup.detect` block likewise updates `vibe-* shims` → `vibe skill`.) Nothing
  parses this token — `orders.sh` resolves the skill from `state-machine.json`, not from
  block text — so the change is cosmetic to the reader and zero-risk functionally. Full
  GWT rewrite deferred to compound; this clause is the durable record of the deviation.

**Progressive disclosure — phase file load**
- GIVEN agent is in `feature.design` state
- WHEN agent invokes `Skill("vibe")`
- THEN `vibe/SKILL.md` loads (routing + orders), and agent follows link to `feature.md` for full design procedure

**install.sh path correctness**
- GIVEN `install.sh` runs into a fresh target repo
- WHEN `vibe-setup/scripts/` has been renamed to `vibe/scripts/`
- THEN `merge-agents.sh` is found at `$TARGET/.agents/skills/vibe/scripts/merge-agents.sh`

**OPEN-2 behavioural preservation**
- GIVEN `feature.verify` has `caveman=full` and `writes=["evidence: ..."]`  and `feature.compound` has `caveman=lite` and broad `writes`
- WHEN the skill folder is consolidated to `vibe/`
- THEN state-machine.json still carries distinct per-state `caveman` and `writes` for each

---

## Non-Goals

- Changing per-state behaviour (caveman levels, write surfaces, delegation chains).
- Changing `orders.sh`, `set-state.sh`, `detect-context.sh`, or other flow scripts.
- Addressing unrelated audit holes (regen-active-rules marker bug, detect-context.sh snapshot, etc.).
- Changing the `.spec/` document format or the spec skill.
