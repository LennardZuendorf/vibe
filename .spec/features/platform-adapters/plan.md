---
type: feature-plan
feature: platform-adapters
sibling: tech.md
parent: ../../plan.md
covers: Claude Code plugin, the three flow hooks, installer
updated: 2026-06-18
---

# Feature: Platform Adapters — Implementation Plan

The Claude Code **plugin**, the three flow **hooks**, and the **installer**. A
closed, deliverable, testable box that wires runtime platforms to a frozen core —
it consumes the orders-in-skills artifact (D12, owned by `vibe-flow`) and the
`merge-agents.sh` artifact (owned by `agent-instructions`); it builds neither.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

**Feature gate:** Starts when `agent-instructions` is `DONE` (root [plan.md](../../plan.md)
Feature Sequence). That transitively requires `vibe-flow` DONE, so D12 orders blocks
already exist in the `vibe-*` skills and `merge-agents.sh` already exists — the hooks
and installer consume those frozen artifacts. No cross-feature unit edges.

Unit IDs are `platform-adapters/n` — assigned once, never renumbered.

---

## Requirements Trace

| ID | Requirement area | Units |
|---|---|---|
| R1, R2, R4 | Adapter files mirror core, define no separate layout (provisioning delegated to `agent-instructions`) | — |
| R3 | Hooks read `.agents/flow` | platform-adapters/1 |
| R7, R8 | Three hooks + wiring; policy lives once in `detect-context.sh` | platform-adapters/1, platform-adapters/2, platform-adapters/3, platform-adapters/4 |
| R9 | Graceful degrade (exit 0); warn-first blocks | platform-adapters/1, platform-adapters/2, platform-adapters/3 |
| R6 | Installable Claude Code plugin | platform-adapters/5 |
| R5 | Installer merges with diff, never blind-overwrite | platform-adapters/6 |

---

## Validation Summary

Validated against the repo on 2026-06-08 (spec `validate.sh` clean).

**Already exists (do NOT rebuild):**
- `.agents/flow/scripts/detect-context.sh` with both `snapshot` and `decide <path>
  [state]` → `allow | warn:<r> | block:<r>`; the three hard blocks are coded here.
- `set-state.sh` (sole `state.json` writer), `validate-state.sh`,
  `regen-active-rules.sh` (symlink-safe, realpath-deduped).
- `AGENTS.md`, `CLAUDE.md` (with `vibe:active-rules`), `.claude/commands/flow.md`.
- `.gitignore` ignores the mutable cursor `.agents/flow/state.json`.

**Consumed from upstream features (frozen before this feature starts):**
- D12 orders blocks in each `vibe-*` skill + `inject: null` on skill states (from
  `vibe-flow`). The inject hook reads the linked skill's orders, not an inline string.
- `merge-agents.sh` marker-aware merge (from `agent-instructions`) for the installer.

**Delivered (2026-06-18):**
- `.claude/hooks/` — `user-prompt-submit-inject.sh` (over `orders.sh`),
  `pre-tool-use-guard.sh` (over `detect-context.sh decide`), `stop-gate.sh`
  (warn-only smells) + `hooks.json` wiring (units 1–4).
- `.claude-plugin/plugin.json` — manifest bundling the `/flow` command + hooks
  via `${CLAUDE_PLUGIN_ROOT}` (unit 5). Skills/flow core ship as project files via
  `install.sh` (no `skills` manifest field exists; `../` is banned in component paths).
- `install.sh` — copy core, seed+gitignore cursor, delegate `AGENTS.md` merge,
  opt-in adapter symlinks, print plugin-registration guidance; idempotent (unit 6).
- Hook behaviour (block/warn/allow/graceful) + installer + merge dogfooded in
  `tests/adapters/run.sh` (unit 7); all `Stop` predicates shipped warn-first.

**Timeline:** 1–2 sessions. Risk: Medium (hook exit-code semantics, PreToolUse stdin).

---

## Decided (inherited from specs — do not relitigate)

- **Hooks are thin shells.** No allow/warn/block logic in any hook; it lives once in
  `detect-context.sh decide`. `state-machine.json` is a link table (orders sourced
  from skills via D12). (R8, tech.md "Earn the teeth".)
- **Earn the teeth, warn-first.** Ship inject (guide) + the three already-coded hard
  blocks (guard) + Stop smells as warn-only (gate). Promote any Stop predicate to
  blocking only after dogfood. (R7, R9.)
- **Graceful degrade = exit 0.** Missing script, missing `jq`, unreadable state →
  exit 0, never end a session. (R9.)
- **Relocatable.** `hooks.json` references scripts via `${CLAUDE_PLUGIN_ROOT}`; the
  plugin owns no state and no spec layout. (R4.)
- **Static orders.** The inject hook emits the linked skill's per-state orders
  verbatim (D12); `<feature>` interpolation is the only allowed substitution. (D10.)

### Resolved (2026-06-18)

- [x] **OPEN-3 install mode.** Shipped **copy** of core `.agents/**` + Claude adapter;
  `merge-agents.sh` handles `AGENTS.md` inside markers; cursor seeded from the example
  and gitignored; opt-in adapter symlinks via `--adapters`. `install.sh` refuses to
  run against the source repo and is idempotent.
- [x] **PreToolUse stdin contract.** Guard reads `tool_input.file_path` (falls back to
  `tool_input.notebook_path` for `NotebookEdit`); `block:` → stderr + **exit 2**;
  `warn:`/`allow` → exit 0. Confirmed against the hook-development reference and
  exercised in `tests/adapters/run.sh`.
- [x] **Closes root OPEN-4 / OPEN-7** (hook strictness): shipped warn-first with only
  the three pre-existing `detect-context.sh` hard blocks active; every `Stop` predicate
  is warn-only and carries a `TODO(earn-the-teeth)` promotion note. No new blocks.

---

## Units

| ID | Seq | Summary | Depends | Status |
|---|---:|---|---|---|
| platform-adapters/1 | 1 | Inject hook (`UserPromptSubmit`) — emits linked-skill orders | — | DONE |
| platform-adapters/2 | 2 | Guard hook (`PreToolUse`) — verdict translator over `detect-context.sh` | — | DONE |
| platform-adapters/3 | 3 | Gate hook (`Stop`) — warn-only smells | — | DONE |
| platform-adapters/4 | 4 | `hooks.json` wiring | platform-adapters/1, platform-adapters/2, platform-adapters/3 | DONE |
| platform-adapters/5 | 5 | `.claude-plugin/plugin.json` manifest | platform-adapters/4 | DONE |
| platform-adapters/6 | 6 | `install.sh` (delegates merge to `agent-instructions`) | platform-adapters/5 | DONE |
| platform-adapters/7 | 7 | Dogfood the hooks; earn-the-teeth review | platform-adapters/6 | DONE |

---

### platform-adapters/1 — Inject hook (`user-prompt-submit-inject.sh`)

**Goal:** on `UserPromptSubmit`, resolve `<flow>.<phase>` from `state.json`
(default `idle`), follow the state's `skill` link, extract that skill's per-state
orders block (D12 — already present), and print it so it rides the user message.
For `idle` only, print the machine's inline fallback. Interpolate `<feature>` only.

**Requirements:** R3, R7, R9

**Dependencies:** — (consumes `vibe-flow` D12, frozen before this feature starts)

**Done when:** for a cursor in `feature.impl` the hook prints that state's orders
(sourced from `vibe-feature`) byte-for-byte; `idle` prints the inline fallback; no
state file prints the idle fallback; never non-zero (missing `jq`/state/skill → 1-line
fallback, exit 0).

---

### platform-adapters/2 — Guard hook (`pre-tool-use-guard.sh`)

**Goal:** on `PreToolUse` (matcher `Edit|Write|NotebookEdit`), read stdin, extract
`tool_input.file_path`, call `detect-context.sh decide <path>`, translate
`block:<r>` → stderr + **exit 2**; `warn:<r>` → stderr, exit 0; `allow` → exit 0.
No policy logic in the hook (R8).

**Requirements:** R7, R8, R9

**Dependencies:** —

**Done when:** a Write to `.spec/lessons.md` outside `*.compound` exits 2 with the
reason; a Write to `src/foo` outside an impl state warns but exits 0; a legal-state
Write exits 0; corrupt/missing stdin or missing `detect-context.sh`/`jq` exits 0.

---

### platform-adapters/3 — Gate hook (`stop-gate.sh`)

**Goal:** on `Stop`, run end-of-turn smell checks via `detect-context.sh` snapshot
(stuck phase; `impl` touched `src/**` with no `tests/**`; `verify` without review;
cursor not advanced). Emit warnings to stderr; **always exit 0** (warn-only).

**Requirements:** R7, R9

**Dependencies:** —

**Done when:** each smell prints a distinct warning when synthesized; the hook never
returns non-zero; each predicate carries a TODO noting it is promotion-eligible after
dogfood.

---

### platform-adapters/4 — Hook wiring (`hooks.json`)

**Goal:** map `UserPromptSubmit` → inject, `PreToolUse (Edit|Write|NotebookEdit)` →
guard, `Stop` → gate, each via `${CLAUDE_PLUGIN_ROOT}/.claude/hooks/<script>`;
`chmod +x` all three.

**Requirements:** R7, R8

**Dependencies:** platform-adapters/1, platform-adapters/2, platform-adapters/3

**Done when:** `hooks.json` is valid JSON, each event points at an existing
executable script, and paths are plugin-root-relative (no absolute paths).

---

### platform-adapters/5 — Plugin manifest (`.claude-plugin/plugin.json`)

**Goal:** declare name (`vibe`), version, description; bundle the `/flow` command,
the `vibe-*` + `spec` skills, and `hooks/hooks.json`. Owns no cursor and no spec
layout.

**Requirements:** R6

**Dependencies:** platform-adapters/4

**Done when:** the manifest is valid against the Claude Code plugin schema and a
plugin install discovers the command, all skills, and the three hooks.

---

### platform-adapters/6 — Installer (`install.sh`)

**Goal:** copy core `.agents/flow/**` and `.agents/skills/**` into a target repo
(default copy, OPEN-3); seed `state.json` from `state.example.json` if absent;
delegate `AGENTS.md` merge to `agent-instructions` (`merge-agents.sh`); create
adapter symlinks per user choice; register the plugin. Idempotent.

**Requirements:** R5

**Dependencies:** platform-adapters/5

**Done when:** running it on a fresh sandbox repo yields a working flow core +
adapters with no manual path correction, never blind-overwrites user content outside
managed markers, and re-running changes nothing.

---

### platform-adapters/7 — Dogfood the hooks

**Goal:** run a `quick` flow and a short `feature` arc with the hooks live; record
which warnings fire, which blocks trip, and any false positives; decide per Stop
predicate whether it earned promotion warn → block.

**Requirements:** R9

**Dependencies:** platform-adapters/6

**Done when:** at least one real lesson is recorded and the warn/block strength of
each predicate is an explicit, justified decision. Feeds root dogfood.

---

## Critical Path

```text
platform-adapters/1 ─┐
platform-adapters/2 ─┼─> /4 ─> /5 ─> /6 ─> /7
platform-adapters/3 ─┘
```

Units 1–3 are independent thin shells (built in any order); unit 4 wires them; 5–6
package and install; 7 dogfoods and decides which teeth to keep. Whole-feature gate:
`agent-instructions` DONE (and transitively `vibe-flow` DONE) before any unit starts.

---

## Progress

| Unit | Status |
|---|---|
| platform-adapters/1 | DONE |
| platform-adapters/2 | DONE |
| platform-adapters/3 | DONE |
| platform-adapters/4 | DONE |
| platform-adapters/5 | DONE |
| platform-adapters/6 | DONE |
| platform-adapters/7 | DONE |

---

## Legacy aliases

One-time map for git grep (old IDs → `feature/n`); do not use for new work. The old
`U8` (D12) moved into the `vibe-flow` feature and is no longer owned here. `U1` =
`platform-adapters/1`, `U2` = `platform-adapters/2`, `U3` = `platform-adapters/3`,
`U4` = `platform-adapters/4`, `U5` = `platform-adapters/5`, `U6` =
`platform-adapters/6`, `U7` = `platform-adapters/7`.
