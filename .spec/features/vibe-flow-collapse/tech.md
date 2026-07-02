---
type: feature-tech
feature: vibe-flow-collapse
sibling: product.md
parent: ../../tech.md
updated: 2026-06-29
---

# Feature: vibe Flow Collapse — Architecture

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)
**Related:** [../vibe-flow/tech.md](../vibe-flow/tech.md), [../vibe-skill-consolidation/tech.md](../vibe-skill-consolidation/tech.md)

---

## New Layout

```text
.agents/
└── skills/
    ├── spec/
    └── vibe/
        ├── SKILL.md
        ├── setup.md … amend.md       (phase files, unchanged)
        ├── state-machine.json        ← moved from .agents/flow/
        ├── state.example.json        ← moved from .agents/flow/
        ├── state.json                ← cursor (gitignored, runtime); paired with state-machine.json
        ├── scripts/
        │   ├── orders.sh detect-context.sh set-state.sh
        │   ├── validate-state.sh regen-active-rules.sh check-skills.sh
        │   └── merge-agents.sh        (already here)
        └── reference/                 (adapters.json, templates/ — unchanged)
```

`.agents/flow/` is **removed**. After this feature `.agents/` contains only `skills/`.

`.claude/` is unchanged in shape — hooks, `/flow` command, `plugin.json` stay (Claude
plugin requirement). They are thin shells repointed at `vibe/scripts/`.

---

## Path Resolution Changes (the non-trivial part)

Every script derives its paths from `SCRIPT_DIR` via `cd ..` chains. Moving from
`.agents/flow/scripts/` to `.agents/skills/vibe/scripts/` adds one directory level,
so the chains shift. There are three distinct fixes — this is **not** a find-replace.

Anchors at the new location: `SCRIPT_DIR = .agents/skills/vibe/scripts`.
- skill dir = `SCRIPT_DIR/..` = `.agents/skills/vibe` → holds `state-machine.json`, `state.json`, `state.example.json`
- skills dir = `SCRIPT_DIR/../..` = `.agents/skills` → holds `vibe/`, `spec/`
- repo root = `SCRIPT_DIR/../../../..` (four ups: scripts→vibe→skills→.agents→root)

| Script | Vars used | Old | New | Fix class |
|---|---|---|---|---|
| `set-state.sh` | `FLOW_DIR`→MACHINE,STATE | `FLOW_DIR=SCRIPT_DIR/..` (=`.agents/flow`) | `SKILL_DIR=SCRIPT_DIR/..` (=`vibe/`) | rename only; MACHINE/STATE follow |
| `validate-state.sh` | `FLOW_DIR`→MACHINE,STATE; example hint | same | same | rename only |
| `detect-context.sh` | `FLOW_DIR`→MACHINE,STATE; **cursor block pattern** | same + literal `.agents/flow/state.json` (line ~79) | `SKILL_DIR`; pattern → `.agents/skills/vibe/state.json` | rename **+ block path (R9)** |
| `orders.sh` | `AGENTS_DIR`→`$AGENTS_DIR/skills/$SKILL/SKILL.md` (line 94) | `AGENTS_DIR=SCRIPT_DIR/../..` (=`.agents`) | `SKILLS_DIR=SCRIPT_DIR/../..` (=`.agents/skills`); lookup `$SKILLS_DIR/$SKILL/SKILL.md` | **drop `skills/` segment** + fallback-text path |
| `check-skills.sh` | `AGENTS_DIR`→`$AGENTS_DIR/skills/spec/SKILL.md` (line 91/94) | same as orders | `SKILLS_DIR`; lookup `$SKILLS_DIR/spec/SKILL.md` | **drop `skills/` segment** |
| `regen-active-rules.sh` | `REPO_ROOT=FLOW_DIR/../..` (3 ups, line 23) | lands repo root | needs 4 ups: `SCRIPT_DIR/../../../..` | **deeper by one level** |

The `AGENTS_DIR` two-up chain is unchanged in *length* but now lands on `.agents/skills`
instead of `.agents`; that is exactly why the `skills/` segment must be dropped from the
lookups (else `.agents/skills/skills/vibe/SKILL.md` — the broken case). `REPO_ROOT` is the
only chain that lengthens.

Also update the in-script comment/usage/fallback strings that hard-code `.agents/flow/...`
(e.g. `orders.sh` `GENERIC_FALLBACK`, `detect-context.sh` header, `set-state.sh` header).

---

## Adapter Repoint (`.claude/`)

| File | Old path | New path |
|---|---|---|
| `hooks/user-prompt-submit-inject.sh` | `$ROOT/.agents/flow/scripts/orders.sh` | `$ROOT/.agents/skills/vibe/scripts/orders.sh` |
| `hooks/pre-tool-use-guard.sh` | `$ROOT/.agents/flow/scripts/detect-context.sh` | `$ROOT/.agents/skills/vibe/scripts/detect-context.sh` |
| `hooks/stop-gate.sh` | `$ROOT/.agents/flow/scripts/detect-context.sh` | `$ROOT/.agents/skills/vibe/scripts/detect-context.sh` |
| `hooks/hooks.json` | description: "Thin shells over .agents/flow/scripts/" | "…over .agents/skills/vibe/scripts/" |
| `commands/flow.md` | reads `.agents/flow/state.json`, `state-machine.json`; runs `.agents/flow/scripts/set-state.sh` | `.agents/skills/vibe/...` |

Hooks keep `${CLAUDE_PLUGIN_ROOT}` for their own location and `${CLAUDE_PROJECT_DIR}`
for the project core — only the `.agents/...` suffix changes.

---

## install.sh Changes

The cursor snapshot/restore/seed/gitignore logic is the regression-sensitive part
(active-rule: *an installer must preserve per-project runtime state across a re-copy*).
The pattern is preserved verbatim; only paths move.

| Lines | Change |
|---|---|
| 8, 10, 16–19 (comments) | drop the separate `.agents/flow/**` mention; cursor path → `.agents/skills/vibe/state.json` |
| 51–68 (copy + snapshot/restore) | remove `cp -R "$SRC/.agents/flow"` (line 61); snapshot/restore target cursor at `.agents/skills/vibe/state.json`. `cp -R .agents/skills` now also carries the source's gitignored `state.json` if present — the existing restore-or-`rm` branch already neutralises that (restore target's, else `rm` the inherited one) |
| 77–79 (chmod) | drop `.agents/flow/scripts/*.sh`; keep `skills/spec` + `skills/vibe` script chmod |
| 82–85 (seed) | `.agents/skills/vibe/state.json` from `.agents/skills/vibe/state.example.json` |
| 88–92 (gitignore) | ignore `.agents/skills/vibe/state.json` |
| 95 (MERGE) | already `skills/vibe/scripts/merge-agents.sh` — **no change** |
| 119 (final note) | "hooks read .agents/flow" → "…read .agents/skills/vibe" |

---

## .gitignore

```diff
-.agents/flow/state.json
+.agents/skills/vibe/state.json
```

`state-machine.json` and `state.example.json` stay tracked (versioned); only the
cursor is ignored.

---

## Migration Reference Inventory

Load-bearing code/config (must change for correctness):

| File | Refs | Change |
|---|---|---|
| `.agents/flow/scripts/*.sh` (×6) | self-location + literals | move to `vibe/scripts/`; apply per-script fix (table above) |
| `.agents/flow/state-machine.json`, `state.example.json` | — | move to `vibe/` |
| `.claude/hooks/*.sh` (×3) + `hooks.json` | 7 | repoint to `vibe/scripts/` |
| `.claude/commands/flow.md` | 3 | repoint to `vibe/` |
| `.agents/skills/vibe/SKILL.md` | 3 | D12 preamble `orders.sh` path; `setup.detect` + `setup.apply` orders blocks (injected verbatim — one-time content change, byte-stable after) |
| `.agents/skills/vibe/setup.md` | 8 | scaffold steps (`set-state.sh`, ensure-`.agents/flow/`-exists, seed cursor, gitignore, `regen`/`check-skills`) → new layout |
| `.agents/skills/vibe/{feature,compound,amend,verify,strategy,quick}.md` | 12 | `Read .agents/flow/state.json` + `set-state.sh`/`detect-context.sh`/`regen-active-rules.sh` invocations |
| `install.sh` | ~15 | cursor logic + paths (table above) |
| `.gitignore` | 1 | cursor path |
| `tests/flow/run.sh` | 4 | script + machine paths |
| `tests/adapters/run.sh` | 6 | install + hook path assertions |

Docs (load-bearing now; `.spec/**` batched at compound):

| File | Refs | When |
|---|---|---|
| `AGENTS.md` | 19 | this feature (canonical guide) — incl. Dogfood table, Repo layout, Commands, Write invariants |
| `README.md` | 7 | this feature |
| `reference/templates/AGENTS.md` | 18 | this feature (shipped template) |
| `.spec/{tech,product,plan,design,lessons}.md`, `.spec/features/**` | ~60 | **compound** (promote + batch path update) |

---

## What Does NOT Change

- Per-state fields in `state-machine.json` (`caveman`, `writes`, `delegates`, `next`, `exit`) and the cursor JSON shape (`{flow, phase, feature, updated}`).
- `orders.sh` resolution **contract**: resolve the linked skill's `SKILL.md`, extract `<!-- vibe:orders:<state> -->`, interpolate `<feature>` only, byte-stable output. Only the resolved *path* changes.
- The three hard write-blocks in `detect-context.sh decide` (lessons.md / root specs / cursor) — the cursor pattern's *path string* updates, the policy does not.
- Skill phase-file **procedures** (the steps each describes) — only the `.agents/flow/...` path/command tokens within them change (R13). The `spec` skill bundle.

---

## Test Compatibility

- `tests/flow/run.sh` — update the paths it invokes (`orders.sh`, `set-state.sh`, `state-machine.json`, `state.example.json`); assertions on orders output stay green (byte-identical).
- `tests/adapters/run.sh` — update install/hook path expectations; the *live-cursor-survives-re-install* test (R8) must pass against `.agents/skills/vibe/state.json`.
- New assertion: after install, `.agents/flow/` does not exist and `.agents/` lists only `skills/`.
