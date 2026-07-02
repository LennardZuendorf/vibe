---
type: feature-plan
feature: vibe-skill-consolidation
sibling: tech.md
parent: ../../plan.md
covers: consolidate 7 vibe-* skill dirs into 1 vibe/ skill dir
updated: 2026-06-29
---

# Feature: vibe Skill Consolidation — Implementation Plan

Consolidate seven `vibe-*` skill directories into one `vibe/` directory following
agentskills.io progressive-disclosure model. All per-state behaviour preserved in
`state-machine.json`; only layout and `skill` link field change.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Note:** This feature supersedes OPEN-2 on folder structure. OPEN-2 behavioural
protection (distinct `writes`/`caveman` per state) is preserved in `state-machine.json`.
The root `plan.md` OPEN-2 entry updates at compound.

---

## Requirements Trace

| Requirement | Unit(s) |
|---|---|
| R1 — one vibe/ dir replaces seven | vibe-skill-consolidation/1, /5 |
| R2 — lean SKILL.md with orders blocks | vibe-skill-consolidation/1 |
| R3 — phase files for procedural content | vibe-skill-consolidation/2 |
| R4 — per-state fields in machine unchanged | vibe-skill-consolidation/3 (verify) |
| R5 — orders.sh no behavioural change | vibe-skill-consolidation/3 (verify) |
| R6 — state-machine.json skill fields → "vibe" | vibe-skill-consolidation/3 |
| R7 — install.sh path updates | vibe-skill-consolidation/4 |
| R8 — cross-reference updates | vibe-skill-consolidation/4 |
| R9 — tests/flow/run.sh passes | vibe-skill-consolidation/3, /5 |

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| vibe-skill-consolidation/1 | 1 | Create vibe/SKILL.md — lean entry + routing + all 14 orders blocks | — | DONE |
| vibe-skill-consolidation/2 | 2 | Create 7 phase files (setup/strategy/feature/quick/verify/compound/amend.md) | /1 | DONE |
| vibe-skill-consolidation/3 | 3 | Update state-machine.json: all skill fields → "vibe"; verify orders.sh + run tests | /2 | DONE |
| vibe-skill-consolidation/4 | 4 | Update all cross-references (install.sh, AGENTS.md, set-state.sh, spec skill refs) | /3 | DONE |
| vibe-skill-consolidation/5 | 5 | Delete 7 old vibe-* dirs; run validate.sh + tests/flow/run.sh green | /4 | DONE |

---

### vibe-skill-consolidation/1 — Create vibe/SKILL.md

**Goal:** Create `.agents/skills/vibe/SKILL.md` as lean entry. Contains:
- Frontmatter: `name: vibe`, consolidated description, `user-invocable: true`,
  `argument-hint: "[setup|strategy|feature|quick|verify|compound|amend]"`,
  `allowed-tools`, `compatibility`
- Body: routing table (argument → phase file) + all 14 `<!-- vibe:orders:<state> -->`
  blocks verbatim from current skill bodies, plus `idle` inline block reference
- Target: under 200 lines

Also move `vibe-setup/scripts/` → `vibe/scripts/` and
`vibe-setup/reference/` → `vibe/reference/` (filesystem move, not copy).

**Done when:** `vibe/SKILL.md` exists, frontmatter valid, all 14 orders blocks
present and byte-identical to current per-skill content, scripts/ and reference/
subdirs exist under vibe/.

---

### vibe-skill-consolidation/2 — Create phase files

**Goal:** Create 7 phase files alongside `vibe/SKILL.md`:

| File | Content source |
|---|---|
| `vibe/setup.md` | `vibe-setup/SKILL.md` body (Procedure + Rules sections) |
| `vibe/strategy.md` | `vibe-strategy/SKILL.md` body |
| `vibe/feature.md` | `vibe-feature/SKILL.md` body |
| `vibe/quick.md` | `vibe-quick/SKILL.md` body |
| `vibe/verify.md` | `vibe-verify/SKILL.md` body |
| `vibe/compound.md` | `vibe-compound/SKILL.md` body |
| `vibe/amend.md` | `vibe-amend/SKILL.md` body |

Each phase file: plain markdown, no frontmatter required. Update internal
cross-references from `vibe-verify` / `vibe-compound` etc. to `verify.md` /
`compound.md` within the same skill dir.

**Done when:** 7 phase files exist under `vibe/`; cross-references within them
are consistent (no references to old `vibe-{name}` dirs that no longer exist).

---

### vibe-skill-consolidation/3 — Update state-machine.json + verify tests

**Goal:** Update `state-machine.json`:
- All 14 `skill` fields: `"vibe-*"` → `"vibe"`
- `idle` inline `inject` text: update user-facing skill name list to `"vibe"` or
  equivalent routing hint

Verify:
1. `bash .agents/flow/scripts/orders.sh` for each of the 14 states produces output
2. `bash tests/flow/run.sh` — all 26 tests green

**Done when:** `orders.sh <state>` works for all states; `tests/flow/run.sh` exits 0.

---

### vibe-skill-consolidation/4 — Update cross-references

**Goal:** Update all files that reference old `vibe-{name}` dirs by path or by
user-visible name. See migration inventory in `tech.md`:

- `install.sh` lines 79, 95: `vibe-setup/scripts/` → `vibe/scripts/`
- `AGENTS.md`: prose refs to `vibe-*` skills → `vibe` where appropriate; routing
  table entry for "Set up or repair harness" → `vibe/SKILL.md`; file layout block
- `.agents/flow/scripts/set-state.sh:54`: error message "vibe-amend" → updated text
- `.agents/skills/spec/feature.md:58`: "vibe-quick" → "vibe"
- `.agents/skills/spec/README.md:68`: "vibe-quick" → "vibe"
- `.spec/tech.md`: skill listing — replace `vibe-*/` with `vibe/`
- `.spec/features/vibe-flow/tech.md`: Files block — update skill listing

**Done when:** `grep -r "vibe-amend\|vibe-compound\|vibe-feature\|vibe-quick\|vibe-strategy\|vibe-verify" .agents/ install.sh .spec/` returns only
the old vibe-* dirs themselves (which will be deleted in /5), or intentional
archive/historical references with a comment.

**Note:** Do NOT update the managed marker comment in `AGENTS.md` header —
`<!-- Managed by vibe-setup (merge-agents.sh) -->` is a historical marker identity,
not a path reference. Evaluate case by case.

---

### vibe-skill-consolidation/5 — Delete old dirs + final validation

**Goal:** Remove the 7 old skill directories:

```bash
rm -rf .agents/skills/vibe-amend \
       .agents/skills/vibe-compound \
       .agents/skills/vibe-feature \
       .agents/skills/vibe-quick \
       .agents/skills/vibe-setup \
       .agents/skills/vibe-strategy \
       .agents/skills/vibe-verify
```

Then run full validation:
- `bash .agents/skills/spec/scripts/validate.sh` — 0 errors
- `bash tests/flow/run.sh` — all 26 tests green
- `bash tests/adapters/run.sh` — green (install.sh path fix verified)

**Done when:** All three test suites exit 0; no `vibe-{name}` directories remain;
`ls .agents/skills/` shows only `spec/` and `vibe/`.

---

## Progress

| Unit | Status |
|---|---|
| vibe-skill-consolidation/1 | DONE |
| vibe-skill-consolidation/2 | DONE |
| vibe-skill-consolidation/3 | DONE |
| vibe-skill-consolidation/4 | DONE |
| vibe-skill-consolidation/5 | DONE |
