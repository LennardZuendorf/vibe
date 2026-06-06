---
type: feature-plan
feature: platform-adapters
sibling: tech.md
parent: ../../plan.md
covers: M4 Stage 2 — Claude Code plugin, the three flow hooks, installer
updated: 2026-06-04
---

# Feature: Platform Adapters — Implementation Plan

Buildable plan for the remaining platform-adapters work: the Claude Code
**plugin**, the three flow **hooks**, and the **installer**. This is the
Stage-2 "earn the teeth" layer the root plan ([../../plan.md](../../plan.md))
tracks as **M4 PARTIAL → DONE**.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md) (R5–R9)
**Architecture:** [tech.md](tech.md)

Unit IDs (`U1`…`U7`) are **stable**: they never change on reorder or split, so
`impl`/`verify` can cite them and survive re-planning (D9).

---

## Validation Summary

Validated against the repo on 2026-06-04 (spec `validate.sh` clean; build-state
audit via subagent).

**Already exists (do NOT rebuild):**
- `.agents/flow/scripts/detect-context.sh` with **both** modes the hooks need:
  `snapshot` (JSON state) and `decide <path> [state]` → `allow | warn:<r> |
  block:<r>`. The three hard blocks are already coded here.
- `.agents/flow/scripts/set-state.sh` (sole `state.json` writer),
  `validate-state.sh` (cursor sanity), `regen-active-rules.sh` (digest).
- `.agents/flow/state-machine.json` — all 15 states carry a `skill` link, a
  `caveman` level, and (today) a frozen `inject` string. **U8 replaces those
  strings with the linked skill's orders** (D12); after U8 the inject hook reads
  the linked skill, not an inline string.
- `AGENTS.md`, `CLAUDE.md` (with the `vibe:active-rules` managed block),
  `.claude/commands/flow.md`. All seven `vibe-*` skills + `spec` skill.
- `.gitignore` already ignores the mutable cursor `.agents/flow/state.json`.

**Must build:**
- Skill-as-inject-source restructure (D12): per-state orders blocks in each
  `vibe-*` skill + state-machine relink (U8). **Prerequisite for U1.**
- `.claude/hooks/` — three thin shell hooks + `hooks.json` wiring (U1–U4).
- `.claude-plugin/plugin.json` — the plugin manifest that bundles command +
  skills + hooks (U5).
- `install.sh` — copy core, merge-with-diff adapters, register the plugin (U6).
- Dogfood + earn-the-teeth review of the hooks (U7), feeding root **M5**.

**Timeline:** 1–2 sessions. Risk: Medium (hook exit-code semantics and the
PreToolUse stdin contract are the only real unknowns).

---

## Critical Architecture Decisions

### Decided (inherited from specs — do not relitigate)
- **D12 — skill shim is the inject source.** The per-turn orders live in each
  `vibe-*` skill, not in a hand-written `inject` string. The state's `skill` field
  is the link; the inject hook pulls the linked skill's per-state orders. Removes
  the inject↔skill duplication (one source of truth). See root
  [../../plan.md](../../plan.md) D12 and [tech.md](tech.md) "Prompt Injection".
- **Hooks are thin shells.** No allow/warn/block logic in any hook; it lives once
  in `detect-context.sh decide`. Inject logic lives once in `state-machine.json`.
  (R8, tech.md "Earn the teeth").
- **Earn the teeth, warn-first.** Ship inject (guide) + the three already-coded
  hard blocks (guard) + Stop smells as **warn-only** (gate). Promote any Stop
  predicate to blocking only after M5 dogfooding. (R7, R9, product "earn the teeth".)
- **Graceful degrade = exit 0.** Missing script, missing `jq`, unreadable state →
  exit 0, never end a session. (R9.)
- **Relocatable.** `hooks.json` references scripts via `${CLAUDE_PLUGIN_ROOT}`;
  the plugin owns no state and no spec layout. (tech.md "No new state".)
- **Static orders.** The inject hook emits the linked skill's per-state orders
  verbatim (D12); it templates nothing turn-varying (prompt-cache discipline, D10).
  `<feature>` interpolation is the only allowed substitution.

### To Resolve (surface at the human gate before `impl`)
- [ ] **Orders granularity (U8/U1).** Inject the **whole** linked skill body every
  turn (simplest; heavier per-turn tokens) **or** a small machine-extractable
  per-`<flow>.<phase>` **orders block** the skill exposes (leaner + cache-stable,
  recommended). Decide before U8 fixes the skill format.
- [ ] **OPEN-3 install mode.** Default proposed: **copy** core `.agents/**` files
  (portable, no broken symlinks across machines); **merge-with-diff** for
  `AGENTS.md`/`CLAUDE.md` (R5 — never blind-overwrite); register the Claude Code
  plugin by manifest. Confirm before building U6, or pick symlink for a live-edit
  dogfood loop.
- [ ] **PreToolUse stdin contract.** Confirm the field name Claude Code passes for
  the target path (`tool_input.file_path`) and the block exit code (**exit 2**)
  against the installed Claude Code version before finalizing U2. Spec already
  assumes exit 2 = block.
- [ ] **Closes root OPEN-4 / OPEN-7** (hook strictness) by shipping warn-first with
  only the three pre-existing hard blocks active. No new blocks this milestone.

---

## Implementation Roadmap

| Milestone | Goal | Units | Sessions | Risk |
|---|---|---|---:|---|
| **PA-0** | Skills become the inject source (D12); state machine relinked | U8 | 0.5–1 | Med |
| **PA-1** | The three hooks exist and are wired, thin, and degrade gracefully | U1–U4 | 1 | Med |
| **PA-2** | Plugin packaging + installer | U5–U6 | 0.5–1 | Med |
| **PA-3** | Dogfood the hooks; earn-the-teeth review | U7 | 0.5 | Low |

Critical path: **U8 → U1 → U2 → U3 → U4 → U5 → U6 → U7** (U8 must land before U1,
which consumes the skill orders blocks; U2/U3 are independent of U1 and may be
built in any order; U4 wires whatever exists).

---

## PA-0: Skills Become the Inject Source (D12)

**Goal:** Each `vibe-*` skill owns its per-state orders; the state machine links to
the skill instead of carrying a hand-written `inject` string. Removes the
inject↔skill duplication so behaviour is edited in one place.

### U8 — Skill-as-inject-source restructure
- Give each `vibe-*` skill a machine-extractable per-`<flow>.<phase>` **orders
  block** carrying what the old inject string held (skill, write surface, output
  path, caveman level, next state). Format per the "orders granularity" decision
  above (recommended: a marked block the hook can slice).
- In `state-machine.json`, set `inject` to `null` for every state that owns a skill
  (the `skill` link is now the source). Keep a minimal inline `inject` only for the
  skill-less states `idle` and `amend` (fallback).
- Update `AGENTS.md` / `CLAUDE.md` adapter prose that still describes the
  "frozen inject string" mechanism to the D12 skill-as-source model. (The
  `vibe:active-rules` managed block is untouched — it is generated.)
- Keep the orders content **byte-stable** per state (prompt-cache discipline); the
  only allowed substitution is `<feature>`.
- **Done when:** for any state, the orders extracted from its linked skill match
  what the old `inject` conveyed; `state-machine.json` carries inline strings only
  for `idle`/`amend`; adapter prose no longer references hand-written inject
  strings; `spec validate.sh` stays clean.

**PA-0 done when:** the single source of per-turn orders is the skill shim, and the
state machine is a pure link table (plus the two fallbacks).

---

## PA-1: The Three Hooks

**Goal:** `.claude/hooks/` holds three thin shells over `.agents/flow/scripts/`,
wired through `hooks.json`, each degrading to exit 0 on any missing keystone.

### U1 — Inject hook (`user-prompt-submit-inject.sh`) — depends on U8
- **Event:** `UserPromptSubmit`. **Strength:** guidance (no exit codes that block).
- Resolve current `<flow>.<phase>` from `state.json` (default `idle`); follow the
  state's `skill` link in `state-machine.json`, extract that skill's per-state
  **orders block** (D12), and print it to stdout so it rides the user message
  (post-cache-breakpoint). For skill-less states (`idle`, `amend`) print the
  machine's inline fallback string instead.
- Interpolate `<feature>` from the cursor when present; substitute **nothing** else
  (static-content / prompt-cache discipline).
- Degrade: missing `jq`/`state.json`/`state-machine.json`/skill file → print a
  1-line `state=idle · pick a vibe-* flow` fallback and exit 0. Include the spec's
  1-line caveman fallback when the caveman plugin is absent (root OPEN-6).
- **Done when:** for a cursor in `feature.impl`, the hook prints that state's orders
  (sourced from `vibe-feature`) byte-for-byte; `idle`/`amend` print the inline
  fallback; with no state file it prints the idle fallback; never non-zero.

### U2 — Guard hook (`pre-tool-use-guard.sh`)
- **Event:** `PreToolUse`, matcher `Edit|Write|NotebookEdit`. **Strength:**
  deterministic block on the three invariants only.
- Read the tool payload from stdin, extract the target path (`tool_input.file_path`),
  call `detect-context.sh decide <path>`, and translate:
  `block:<r>` → print reason to stderr, **exit 2**; `warn:<r>` → stderr, exit 0;
  `allow` → exit 0.
- No policy logic in the hook — it is a verdict translator only (R8).
- Degrade: unparsable stdin, missing `detect-context.sh`, missing `jq` → exit 0.
- **Done when:** a Write to `.spec/lessons.md` outside `*.compound` exits 2 with the
  reason; a Write to `src/foo` outside an impl state warns but exits 0; a Write
  during the legal state exits 0; corrupt stdin exits 0.

### U3 — Gate hook (`stop-gate.sh`)
- **Event:** `Stop`. **Strength:** warn-only this milestone (earn the teeth).
- Run end-of-turn smell checks via `detect-context.sh` snapshot: stuck phase,
  `impl` touched `src/**` with no `tests/**`, `verify` entered without review,
  cursor not advanced (`set-state.sh` apparently forgotten). Emit warnings to
  stderr; **always exit 0** (no predicate is blocking yet).
- Leave a TODO marker per predicate noting it is promotion-eligible after M5.
- **Done when:** each smell prints a distinct warning when its condition is
  synthesized, and the hook never returns non-zero.

### U4 — Hook wiring (`hooks.json`)
- Map `UserPromptSubmit → U1`, `PreToolUse (Edit|Write|NotebookEdit) → U2`,
  `Stop → U3`, each referenced via `${CLAUDE_PLUGIN_ROOT}/.claude/hooks/<script>`.
- `chmod +x` all three scripts.
- **Done when:** `hooks.json` is valid JSON, each event points at an existing
  executable script, and paths are plugin-root-relative (no absolute paths).

**PA-1 done when:** the three hooks run from a clean checkout, the inject fires
with the correct linked-skill orders (D12), the guard blocks exactly the three
invariants and warns elsewhere, the gate only warns, and every degradation path
exits 0.

---

## PA-2: Plugin Packaging & Installer

### U5 — Plugin manifest (`.claude-plugin/plugin.json`)
- Declare name (`vibe`), version, description, and bundle the `/flow` command,
  the `vibe-*` + `spec` skills, and `hooks/hooks.json`.
- Owns no cursor and no spec layout — it is packaging, not a second core.
- **Done when:** the manifest is valid against the Claude Code plugin schema and a
  plugin install discovers the command, all skills, and the three hooks.

### U6 — Installer (`install.sh`)
- Copy core `.agents/flow/**` and `.agents/skills/**` into a target repo (default
  **copy**, per OPEN-3); seed `state.json` from `state.example.json` if absent.
- Merge-or-diff `AGENTS.md`/`CLAUDE.md` — never blind-overwrite (R5); preserve the
  `vibe:active-rules` managed block.
- Register the Claude Code plugin via the manifest. Idempotent; safe re-runs.
- **Done when:** running it on a fresh sandbox repo yields a working flow core +
  adapters with no manual path correction, and re-running changes nothing.

**PA-2 done when:** a single plugin install wires command + skills + hooks, and
`install.sh` provisions a target repo non-destructively.

---

## PA-3: Dogfood & Earn the Teeth

### U7 — Dogfood the hooks (feeds root M5)
- Run a `quick` flow and a short `feature` arc with the hooks live; record which
  warnings fire, which blocks trip, and any false positives.
- Decide per Stop predicate whether it earned promotion from warn → block; record
  the decision (and any friction) as a tagged `lessons.md` entry during compound.
- **Done when:** at least one real lesson is recorded and the warn/block strength
  of each predicate is an explicit, justified decision rather than a default.

---

## Critical Path

```text
              U1 ─┐
U8 ─> U1; U2 ─┼─> U4 ─> U5 ─> U6 ─> U7
              U3 ─┘
```

U8 makes the skills the inject source (must land first). U1 consumes it; U2/U3 are
independent shells; U4 wires them; U5–U6 package and install; U7 dogfoods and
decides which teeth to keep. PA-3 closes root **M4** and opens root **M5**.

---

## Progress

| Unit | Description | Status |
|---|---|---|
| U8 | Skills become the inject source (D12) + relink machine | NOT STARTED |
| U1 | Inject hook (consumes U8) | NOT STARTED |
| U2 | Guard hook | NOT STARTED |
| U3 | Gate hook | NOT STARTED |
| U4 | hooks.json wiring | NOT STARTED |
| U5 | plugin.json manifest | NOT STARTED |
| U6 | install.sh | NOT STARTED |
| U7 | Dogfood + earn-the-teeth | NOT STARTED |
