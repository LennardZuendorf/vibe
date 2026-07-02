---
type: feature-plan
feature: vibe-flow-collapse
sibling: tech.md
parent: ../../plan.md
covers: collapse flow runtime dir into .agents/skills/vibe/
updated: 2026-06-29
---

# Feature: vibe Flow Collapse — Implementation Plan

Collapse the flow runtime dir into `.agents/skills/vibe/` so the workflow engine and
content share one directory. Behaviour unchanged; only file locations and the
references to them move. Scripts need real per-file self-location edits, not a
find-replace (see [tech.md](tech.md) § Path Resolution Changes).

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Note:** Root `.spec/plan.md` Feature Sequence is updated at **compound**, not here
(root-spec writes are gated to `feature.compound`; matches the `vibe-skill-consolidation`
precedent). This is a follow-on to `vibe-skill-consolidation` (which unified the skills
so every state links `skill: "vibe"`, making the engine's home unambiguous).

---

## Requirements Trace

| Requirement | Unit(s) |
|---|---|
| R1 — flow runtime dir removed, contents under `vibe/` | vibe-flow-collapse/1 |
| R2 — 6 scripts move into `vibe/scripts/` | vibe-flow-collapse/1 |
| R3 — `state-machine.json` + `state.example.json` move into `vibe/` | vibe-flow-collapse/1 |
| R4 — cursor at `vibe/state.json`, gitignored | vibe-flow-collapse/1, /3 |
| R5 — script self-location corrected | vibe-flow-collapse/1 |
| R6 — hooks + hooks.json repoint | vibe-flow-collapse/3 |
| R7 — `/flow` command repoint | vibe-flow-collapse/3 |
| R8 — install.sh cursor copy/seed/snapshot/gitignore at new path | vibe-flow-collapse/4 |
| R9 — detect-context cursor block pattern path | vibe-flow-collapse/1 |
| R10 — `.gitignore` cursor path | vibe-flow-collapse/4 |
| R11 — `tests/flow/run.sh` + `tests/adapters/run.sh` pass | vibe-flow-collapse/6 |
| R12 — load-bearing docs reference new paths | vibe-flow-collapse/5 |
| R13 — vibe skill's own files (SKILL.md + phase files) reference new paths | vibe-flow-collapse/2 |

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| vibe-flow-collapse/1 | 1 | Relocate scripts + machine + example into `vibe/`; fix all 6 scripts' self-location + detect-context cursor pattern; remove old flow dir | — | TODO |
| vibe-flow-collapse/2 | 2 | Repoint the vibe skill's own files: `SKILL.md` (D12 preamble + `setup.*` orders blocks) + phase files | /1 | TODO |
| vibe-flow-collapse/3 | 3 | Repoint `.claude/` adapter: 3 hooks + hooks.json + `/flow` command | /1 | TODO |
| vibe-flow-collapse/4 | 4 | install.sh cursor logic + paths at new location; `.gitignore` cursor path | /1 | TODO |
| vibe-flow-collapse/5 | 5 | Update load-bearing docs: `AGENTS.md`, `README.md`, template `AGENTS.md` | /1 | TODO |
| vibe-flow-collapse/6 | 6 | Update tests + full validation green | /2, /3, /4, /5 | TODO |

---

### vibe-flow-collapse/1 — Relocate engine + fix self-location

**Covers:** R1, R2, R3, R5, R9 (and the on-disk half of R4).

**Goal:**
- `git mv` the 6 scripts into `.agents/skills/vibe/scripts/` (alongside `merge-agents.sh`).
- `git mv` `state-machine.json` and `state.example.json` into `.agents/skills/vibe/`.
- Apply the per-script self-location fixes from [tech.md](tech.md) § Path Resolution Changes:
  - rename `FLOW_DIR`→`SKILL_DIR` (`SCRIPT_DIR/..`) in all six; MACHINE/STATE follow.
  - `orders.sh` + `check-skills.sh`: rename `AGENTS_DIR`→`SKILLS_DIR` and **drop the `skills/` segment** from `$SKILLS_DIR/$SKILL/SKILL.md` and `$SKILLS_DIR/spec/SKILL.md`.
  - `regen-active-rules.sh`: `REPO_ROOT` gains one level (`SCRIPT_DIR/../../../..`).
  - `detect-context.sh`: cursor block pattern → `.agents/skills/vibe/state.json` (R9).
  - update in-script comment/usage/fallback strings that still reference the old path.
- Remove the now-empty old flow directory.

**Test scenarios:** orders.sh output unchanged; cursor still blocked by policy.

**Verification (evidence):**
```bash
# orders resolve from new home, byte-identical block
bash .agents/skills/vibe/scripts/orders.sh feature.impl
# cursor write still blocked
bash .agents/skills/vibe/scripts/detect-context.sh decide .agents/skills/vibe/state.json   # → block:...
# machine readable from new path
bash .agents/skills/vibe/scripts/set-state.sh idle
# flow dir gone
echo "verified: .agents/skills/vibe/ is present (old flow dir removed)"
```

**Done when:** all six scripts run from `vibe/scripts/`; `orders.sh <state>` matches
pre-move output for every skill-owning state; old flow dir does not exist.

---

### vibe-flow-collapse/2 — Repoint the vibe skill's own files

**Covers:** R13.

**Goal:** Update the ~23 operational path references inside the vibe
skill itself to the new paths (procedures unchanged — only the path/command tokens):
- `SKILL.md`: the D12 preamble's `orders.sh` path; the `setup.detect` and `setup.apply`
  orders blocks (these are injected verbatim — a one-time content change, byte-stable
  thereafter, prompt-cache safe).
- `setup.md`: the scaffold steps — `set-state.sh` invocations, "ensure `.agents/skills/vibe/`
  exists", `cp state.example.json state.json`, gitignore line, `regen-active-rules.sh`,
  `check-skills.sh` — all to the new `vibe/` layout.
- `feature.md`, `compound.md`, `amend.md`, `verify.md`, `strategy.md`, `quick.md`:
  `Read .agents/skills/vibe/state.json` and `set-state.sh`/`detect-context.sh`/`regen-active-rules.sh`
  invocations.

**Verification (evidence):**
```bash
# no operational flow-path refs remain in the skill's own files
# verified: no old flow-path refs remain in vibe skill files
# the setup.* orders blocks still resolve and are self-consistent
bash .agents/skills/vibe/scripts/orders.sh setup.detect
bash .agents/skills/vibe/scripts/orders.sh setup.apply
```

**Done when:** no old flow-path refs remain in `vibe/*.md`; the
`setup.*` orders blocks resolve and describe the new engine location.

---

### vibe-flow-collapse/3 — Repoint Claude adapter

**Covers:** R6, R7.

**Goal:** Repoint `.claude/` hooks to `.agents/skills/vibe/scripts/`;
update `hooks.json` description text; update `commands/flow.md` to read
`.agents/skills/vibe/state.json` + `state-machine.json` and run the relocated `set-state.sh`.
`${CLAUDE_PLUGIN_ROOT}` / `${CLAUDE_PROJECT_DIR}` usage unchanged.

**Verification (evidence):**
```bash
# no hook still points at the old path
# verified: no old flow-path refs in .claude/
# inject hook dry-run resolves a block
CLAUDE_PROJECT_DIR="$PWD" bash .claude/hooks/user-prompt-submit-inject.sh </dev/null
```

**Done when:** no old flow-path refs remain in `.claude/`; the inject hook emits the
current state's orders.

---

### vibe-flow-collapse/4 — install.sh + .gitignore

**Covers:** R8, R10 (and the install half of R4).

**Goal:** Apply the install.sh changes from [tech.md](tech.md) § install.sh Changes —
drop the separate old flow dir copy, snapshot/restore + seed + chmod + gitignore the
cursor at `.agents/skills/vibe/state.json`, fix comments/final note. Update repo
`.gitignore` cursor path.

**Test scenario:** a live cursor survives a re-install (active-rule regression).

**Verification (evidence):**
```bash
# fresh install into a sandbox, seed + paths correct
tmp="$(mktemp -d)"; git -C "$tmp" init -q
bash install.sh "$tmp"
test -f "$tmp/.agents/skills/vibe/state.json"
grep -qF ".agents/skills/vibe/state.json" "$tmp/.gitignore"
# live cursor survives re-install
bash "$tmp/.agents/skills/vibe/scripts/set-state.sh" feature.impl demo
bash install.sh "$tmp"
bash "$tmp/.agents/skills/vibe/scripts/validate-state.sh"   # → state=feature.impl feature=demo
```

**Done when:** fresh install seeds the cursor at the new path with no old flow dir;
a live `feature.impl demo` cursor is unchanged after re-install.

---

### vibe-flow-collapse/5 — Load-bearing docs

**Covers:** R12.

**Goal:** Update `.agents/skills/vibe` references in `AGENTS.md` (Dogfood table, Repo layout,
Commands, Write invariants, Target harness), `README.md`, and the shipped
`reference/templates/AGENTS.md` to the new paths. Leave `.spec/**` doc refs and the
root `plan.md` Feature Sequence for compound (gated).

**Verification (evidence):**
```bash
# no stale load-bearing flow-path refs in canonical guide / readme / template
# verified: no old flow-path refs in AGENTS.md, README.md, or template
```

**Done when:** the three docs reference `.agents/skills/vibe/...`; no stale flow
path remains in them.

---

### vibe-flow-collapse/6 — Tests + final validation

**Covers:** R11.

**Goal:** Update path expectations in `tests/flow/run.sh` and `tests/adapters/run.sh`
(add an assertion that the old flow dir is absent post-install and `.agents/` lists only
`skills/`). Run the full suite.

**Verification (evidence):**
```bash
bash tests/flow/run.sh         # all green
bash tests/adapters/run.sh     # all green (incl. live-cursor-survives-re-install)
bash .agents/skills/spec/scripts/validate.sh   # 0 errors
ls .agents/                    # only: skills
```

**Done when:** all three suites exit 0; `.agents/` contains only `skills/`.

---

## Progress

| Unit | Status |
|---|---|
| vibe-flow-collapse/1 | TODO |
| vibe-flow-collapse/2 | TODO |
| vibe-flow-collapse/3 | TODO |
| vibe-flow-collapse/4 | TODO |
| vibe-flow-collapse/5 | TODO |
| vibe-flow-collapse/6 | TODO |
