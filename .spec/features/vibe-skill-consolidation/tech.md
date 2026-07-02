---
type: feature-tech
feature: vibe-skill-consolidation
sibling: product.md
parent: ../../tech.md
updated: 2026-06-29
---

# Feature: vibe Skill Consolidation — Architecture

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)
**Related:** [../vibe-flow/tech.md](../vibe-flow/tech.md)

---

## New Layout

```text
.agents/skills/
└── vibe/
    ├── SKILL.md          ← lean entry: frontmatter + routing table + all 14 orders blocks
    ├── setup.md          ← setup.detect + setup.apply procedures
    ├── strategy.md       ← strategy.brainstorm + strategy.spec procedures
    ├── feature.md        ← feature.design + feature.plan + feature.impl procedures
    ├── quick.md          ← quick.triage + quick.fix procedures
    ├── verify.md         ← feature.verify + quick.verify procedures
    ├── compound.md       ← feature.compound + strategy.compound procedures
    ├── amend.md          ← amend modifier procedure
    ├── scripts/          ← moved from vibe-setup/scripts/ (merge-agents.sh)
    └── reference/        ← moved from vibe-setup/reference/ (adapters.json, templates/)
```

Replaces:
```text
.agents/skills/
├── vibe-amend/SKILL.md
├── vibe-compound/SKILL.md
├── vibe-feature/SKILL.md
├── vibe-quick/SKILL.md
├── vibe-setup/SKILL.md + scripts/ + reference/
├── vibe-strategy/SKILL.md
└── vibe-verify/SKILL.md
```

---

## What Does NOT Change

Per-state behaviour lives in `state-machine.json`, not in skill folders. The following
fields are **unchanged** by this feature:

| Field | Lives in | Example |
|---|---|---|
| `caveman` | `state-machine.json` per-state | `feature.verify: full`, `strategy.compound: lite` |
| `writes` | `state-machine.json` per-state | `feature.impl: ["src/**", "tests/**"]` |
| `delegates` | `state-machine.json` per-state | `["superpowers:executing-plans", ...]` |
| `next` | `state-machine.json` per-state | `["feature.verify"]` |
| `exit` | `state-machine.json` per-state | `"tests reference plan unit IDs and pass"` |

`orders.sh` behavioural contract is unchanged: it resolves
`$AGENTS_DIR/skills/$SKILL/SKILL.md` and extracts the matching
`<!-- vibe:orders:<state> -->` block. After consolidation `$SKILL` = `"vibe"` for all
14 skill-owning states; `idle` retains its inline `inject` in the machine.

---

## state-machine.json Changes

All 14 `skill` fields update to `"vibe"`:

```json
// Before                              After
"skill": "vibe-setup"       →   "skill": "vibe"
"skill": "vibe-strategy"    →   "skill": "vibe"
"skill": "vibe-compound"    →   "skill": "vibe"
"skill": "vibe-feature"     →   "skill": "vibe"
"skill": "vibe-verify"      →   "skill": "vibe"
"skill": "vibe-quick"       →   "skill": "vibe"
"skill": "vibe-amend"       →   "skill": "vibe"
```

The `idle` state inline `inject` string references old skill names as user-facing
text (`vibe-setup, vibe-strategy,...`) and updates to `vibe` (or the routed phase
file name) for consistency.

---

## install.sh Changes

`install.sh` copies `.agents/skills/` wholesale (line 62: `cp -R`), so the new
`vibe/` dir arrives automatically. Two hardcoded `vibe-setup` paths must update:

```bash
# Line 79 — before:
"$TARGET"/.agents/skills/vibe-setup/scripts/*.sh

# Line 79 — after:
"$TARGET"/.agents/skills/vibe/scripts/*.sh

# Line 95 — before:
MERGE="$TARGET/.agents/skills/vibe-setup/scripts/merge-agents.sh"

# Line 95 — after:
MERGE="$TARGET/.agents/skills/vibe/scripts/merge-agents.sh"
```

---

## Migration Reference Inventory

All references to old skill names that must be updated:

| File | Line(s) | Change |
|---|---|---|
| `.agents/skills/vibe/state-machine.json` | 14 `skill` fields + idle inject text | `vibe-*` → `"vibe"` |
| `install.sh` | 79, 95 | `vibe-setup/scripts/` → `vibe/scripts/` |
| `AGENTS.md` | 41, 44, 57, 90, 93, 95, 108, 133, 211 | prose refs `vibe-*` → `vibe` where appropriate |
| `.agents/skills/vibe/scripts/set-state.sh` | 54 | error msg "Run vibe-amend" → "Run /vibe amend" or similar |
| `.agents/skills/spec/feature.md` | 58 | "vibe-quick" → "vibe" |
| `.agents/skills/spec/README.md` | 68 | "vibe-quick" → "vibe" |
| Phase files (inter-skill cross-refs) | various | "vibe-verify", "vibe-compound" refs → `verify.md`, `compound.md` |
| `.spec/tech.md` | skill listing | add `vibe/` remove `vibe-*/` |
| `.spec/features/vibe-flow/tech.md` | Files block | update skill listing |
| `AGENTS.md` template in `reference/templates/` | 4 | "vibe-setup" in comment stays (it's a marker owner) |

---

## SKILL.md Structure

The consolidated `vibe/SKILL.md` is the lean entry point. Structure:

```markdown
---
name: vibe
description: |
  Vibe workflow skill. Routes to the right phase for the current flow state.
  Trigger on: any vibe flow state, "what should I do", flow navigation, ...
user-invocable: true
argument-hint: "[setup|strategy|feature|quick|verify|compound|amend]"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
---

# vibe — workflow router

[routing table: argument → phase file]

## Orders (D12)

[all 14 <!-- vibe:orders:<state> --> blocks, verbatim from current skill bodies]
```

Target: under 200 lines. Phase files carry the full procedural content.

---

## Test Compatibility

`tests/flow/run.sh` tests orders resolution, state transitions, and graceful
degradation. After consolidation:
- `orders.sh` resolves `vibe/SKILL.md` instead of `vibe-{name}/SKILL.md` — same
  block format, same output for each state.
- `check-skills.sh` checks for the bundled `spec` skill by path
  (`.agents/skills/spec/SKILL.md`) — unchanged.
- All 26 existing tests pass without modification (orders content is preserved
  verbatim; only the containing file path changes).
