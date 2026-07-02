---
type: feature-product
feature: vibe-flow-collapse
sibling: tech.md
parent: ../../product.md
updated: 2026-06-29
---

# Feature: vibe Flow Collapse — Product

Collapse the `.agents/flow/` runtime directory into the `.agents/skills/vibe/`
skill so the workflow engine and its content live in one place. The flow scripts,
`state-machine.json`, and the runtime cursor relocate under `vibe/`; `.agents/flow/`
is removed. Behaviour is unchanged — only file locations and the paths that
reference them move. The Claude hooks and `/flow` command stay in the `.claude/`
adapter (a Claude plugin requirement) and are repointed at the skill's new
`scripts/` path.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)
**Related:** [../vibe-skill-consolidation/product.md](../vibe-skill-consolidation/product.md), [../vibe-flow/product.md](../vibe-flow/product.md)

---

## Background

`vibe-skill-consolidation` merged seven `vibe-*` skill dirs into one `vibe/` skill,
and every state now links to `skill: "vibe"`. With the skills unified, the split
between `.agents/flow/` (engine) and `.agents/skills/vibe/` (content) is the
remaining source of "scattered" layout. The stated goal is **tidiness**: one
directory holding the workflow engine, not two side by side.

Three artifact classes live in `.agents/flow/`, distinguished by who consumes them:

| Artifact | Consumer | Disposition |
|---|---|---|
| 6 scripts (`orders`, `set-state`, `detect-context`, `validate-state`, `regen-active-rules`, `check-skills`) | hooks + skill | move into `vibe/scripts/` (joins `merge-agents.sh`) |
| `state-machine.json`, `state.example.json` | adapters (`/flow`, hooks) + skill | move into `vibe/` |
| `state.json` (runtime cursor) | per-project mutable runtime | move to `vibe/state.json`, gitignored |

What cannot move: the three hooks and the `/flow` command. They are Claude-specific
runtime wiring and must ship in the `.claude/` plugin (active-rule: *a plugin cannot
bundle skills outside ./skills/*; the platform-neutral core ships via `install.sh`).
They are thin shells over the scripts and only need their paths repointed.

The vibe skill's **own** files (`SKILL.md` + the phase files) carry ~23 operational
`.agents/flow/...` references — `Read .agents/flow/state.json`, `set-state.sh`/`detect-context.sh`/`regen-active-rules.sh`
invocations, and the `setup.md` scaffold steps. These are the flow's own operating
instructions; every one breaks when `flow/` is removed, so they move with the feature.
Two of them live inside the `setup.detect`/`setup.apply` **orders blocks** in `SKILL.md`,
which the inject hook emits verbatim — a one-time content change that stays byte-stable
(prompt-cache safe) thereafter.

This feature **supersedes** the root-plan boundary "vibe-flow owns `.agents/flow`": the
engine moves under the vibe skill. Recorded here; the root plan promotes at compound.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | `.agents/flow/` SHALL be removed; its contents relocated under `.agents/skills/vibe/`. After this feature `.agents/` contains only `skills/`. |
| R2 | The six flow scripts SHALL move into `.agents/skills/vibe/scripts/`, alongside `merge-agents.sh`. |
| R3 | `state-machine.json` and `state.example.json` SHALL move into `.agents/skills/vibe/`. |
| R4 | The runtime cursor SHALL live at `.agents/skills/vibe/state.json`, gitignored, paired with `state-machine.json`. |
| R5 | Each script's self-location SHALL be corrected for the new depth; `orders.sh` and `check-skills.sh` skill-dir resolution SHALL resolve `.agents/skills/vibe/SKILL.md`, never `.agents/skills/skills/...`. |
| R6 | The three `.claude/hooks/*.sh` and `hooks.json` SHALL repoint to `.agents/skills/vibe/scripts/`. |
| R7 | The `/flow` command (`.claude/commands/flow.md`) SHALL repoint to the new `state.json`, `state-machine.json`, and `set-state.sh` paths. |
| R8 | `install.sh` SHALL copy, seed, snapshot/restore, and gitignore the cursor at its new path; the *live-cursor-survives-re-install* regression test SHALL still pass. |
| R9 | `detect-context.sh` block pattern for the cursor SHALL match the new `state.json` path. |
| R10 | `.gitignore` SHALL ignore `.agents/skills/vibe/state.json`. |
| R11 | `tests/flow/run.sh` and `tests/adapters/run.sh` SHALL pass after path updates. |
| R12 | Load-bearing docs (`AGENTS.md`, `README.md`, the template `AGENTS.md`) SHALL reference the new paths. `.spec/**` doc references batch-update at compound. |
| R13 | The vibe skill's own files (`SKILL.md` + phase files) SHALL reference the new paths; the `setup.detect`/`setup.apply` orders blocks and the `setup.md` scaffold steps SHALL describe the new engine location. Procedures are otherwise unchanged — only the `.agents/flow/...` path/command tokens update. |

---

## Scope

| Owns | Does not own |
|---|---|
| Relocation of 6 scripts + `state-machine.json` + `state.example.json` + cursor into `vibe/` | Per-state fields in `state-machine.json` (`caveman`, `writes`, `delegates`, `next`, `exit`) |
| Removal of `.agents/flow/` | `orders.sh` resolution **contract** (block format, byte-stable output) |
| Per-script self-location edits (R5) | Hooks living in `.claude/` (stay — Claude plugin requirement) |
| `.claude/hooks/*` + `hooks.json` + `/flow` path repoint | Skill phase-file *procedures* (only the `.agents/flow/...` tokens within them change) |
| `install.sh` cursor copy/seed/snapshot/gitignore at new path | Root `plan.md` Feature Sequence (promote at compound) |
| `detect-context.sh` cursor block pattern path (R9) | The `spec` skill bundle |
| Path/command refs in `vibe/SKILL.md` + phase files, incl. `setup.*` orders blocks (R13) | |
| `.gitignore` cursor path (R10) | |
| `AGENTS.md` / `README.md` / template path refs (R12) | |

---

## GWT Scenarios

**Live cursor survives re-install at new path**
- GIVEN a target repo with a live cursor `.agents/skills/vibe/state.json` (`feature.impl <feature>`)
- WHEN `install.sh` re-runs over it
- THEN the cursor is snapshotted before the copy and restored after — the live `feature.impl <feature>` state is unchanged, not reseeded to idle

**Hook resolves orders from the new path**
- GIVEN the `UserPromptSubmit` inject hook
- WHEN it runs after the move
- THEN it executes `.agents/skills/vibe/scripts/orders.sh` and injects the current state's orders block

**orders.sh output unchanged**
- GIVEN `orders.sh <state>` for any of the 14 skill-owning states
- WHEN scripts and machine live under `vibe/` and self-location is corrected (R5)
- THEN output is byte-identical to the pre-collapse output (only the resolved file path changed, never the block text)

**Engine and content in one directory**
- GIVEN the feature is complete
- WHEN `ls .agents/` runs
- THEN it shows only `skills/`; `ls .agents/skills/vibe/` shows `SKILL.md`, the phase files, `state-machine.json`, `scripts/`, `reference/` (and `state.json` on a live working copy)

**Cursor never tracked**
- GIVEN the relocated cursor `.agents/skills/vibe/state.json`
- WHEN `git status` runs
- THEN `state.json` is ignored; `state-machine.json` and `state.example.json` are tracked

---

## Non-Goals

- Moving the hooks or `/flow` command out of `.claude/` — impossible under the Claude plugin model; they stay and are repointed only.
- Changing the orders block format, the cursor JSON shape, or any per-state behaviour.
- Changing skill *procedures* (the steps each phase file describes) beyond the `.agents/flow/...` path/command tokens they contain; changing the `spec` skill.
- Achieving a fully portable drop-in skill (the gitignored cursor and the plugin-resident hooks are inherent seams; goal here is tidiness, not portability).
