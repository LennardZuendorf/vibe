---
type: feature-plan
feature: content-layer
sibling: tech.md
parent: ../../plan.md
updated: 2026-08-15
---

# Feature: Content Layer — Implementation Plan

Six units: the marker grammar's write half, the resolver, the renderer command,
the shipped default rulesets, the hook + install wiring, and the test matrix.
Delivered on top of js-core, whose engine primitives it consumes unchanged.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** Starts when js-core is `DONE` (root [plan.md](../../plan.md) Feature Sequence, row 12). Does not depend on any other feature's units.

---

## Problem Frame

Injected prose had no single home: the per-turn orders live in `flow/SKILL.md`,
the doctrine in a second block, the standing rules in a static `AGENTS.md`
template — three surfaces, three edit paths, no project override that survives a
reinstall, and no way to add a rule without forking what ships. The fix is to
make injected content DATA: one authoring format, one resolver, one config file
a project owns, and a lint that fails when the result is oversized or broken.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Content is authored once, as data](product.md#requirement-content-is-authored-once-as-data-r1) | content-layer/2 |
| R2 | [One block, two verbosities, many channels](product.md#requirement-one-block-two-verbosities-many-channels-r2) | content-layer/2 |
| R3 | [Machine fields interpolate as typed placeholders](product.md#requirement-machine-fields-interpolate-as-typed-placeholders-r3) | content-layer/2 |
| R4 | [Injection fails shut, never fatal](product.md#requirement-injection-fails-shut-never-fatal-r4) | content-layer/2, content-layer/5 |
| R5 | [Budgets and lints are enforced by a command](product.md#requirement-budgets-and-lints-are-enforced-by-a-command-r5) | content-layer/3, content-layer/6 |
| R6 | [Document channels sync into a managed block](product.md#requirement-document-channels-sync-into-a-managed-block-r6) | content-layer/1, content-layer/3 |
| R7 | [Three default rulesets ship enabled](product.md#requirement-three-default-rulesets-ship-enabled-r7) | content-layer/4, content-layer/5 |

---

## Key Technical Decisions

1. **Additive, not a migration.** The flow orders and doctrine stay where they
   are and are exposed as `{{orders}}` / `{{doctrine}}` placeholders. Rewriting
   them as blocks would have put every byte-parity test in js-core at risk for no
   user-visible gain; a later feature can migrate them behind the placeholders.
2. **The project layer is one file.** `vibe.json` at the repo root, never written
   by install, merged over the shipped defaults. `.vibe/blocks/**` exists for
   long rules that do not belong inline in JSON.
3. **The engine may not name the layout.** The lessons path is config data and the
   markers come from `blocks.mjs`, so `content.mjs` holds no banned ingredient and
   the duplicate-primitive scan covers the new modules with no waiver.
4. **Fail shut, report loudly.** Hooks use `renderChannelSafe` (never throws,
   contributes nothing when broken); `vibe render --check` is where the same
   errors become a non-zero exit.

---

## Global Constraints

- Zero runtime dependencies; Node 18+; every module never throws out of a hook path.
- `node flow/engine/tests/run.mjs` and `bash tests/run.sh` green before any claim of done.
- No engine module may name `.spec`, `.agents/skills/vibe`, a cursor/machine filename, or the `<!--` grammar (`flow/engine/tests/primitives.test.mjs`).

---

## Unit IDs

Units are `content-layer/n`, assigned once and never renumbered.

---

### content-layer/1 — Marker grammar: the write half

**Goal:** `blocks.mjs` can write the grammar it already reads, so no other module spells a marker.

**Requirements:** R6

**Dependencies:** —

**Files:**

```
flow/engine/blocks.mjs              # + blockSpan/stripBlock/renderBlock/upsertBlock
```

**Test scenarios:** append vs replace vs unchanged; content preserved on both sides of an existing block; an unclosed opener changes nothing (reader and writer agree).

**Verification:** `node flow/engine/tests/run.mjs blocks` and the `blocks:` cases in `content.test.mjs`.

---

### content-layer/2 — The resolver

**Goal:** three config layers, block parsing, placeholders, per-channel render, and the lint.

**Requirements:** R1, R2, R3, R4

**Dependencies:** content-layer/1

**Files:**

```
flow/engine/content.mjs             # loadContent/renderChannel/renderChannelSafe/checkContent
```

**Test scenarios:** shipped-only render; `blocks` replaces, `add`/`remove` edit; `enabled:false`; inline and file-backed project blocks; `.vibe/blocks` override; project-declared channel; summary vs body mode; budget overflow; unknown id; malformed/array `vibe.json`; hostile ctx; every placeholder including `{{lessons:TAG}}` and an unresolved one.

**Verification:** `node flow/engine/tests/run.mjs content`.

---

### content-layer/3 — `vibe render`

**Goal:** the CLI: compose, list, check, and sync a document channel into its managed block.

**Requirements:** R5, R6

**Dependencies:** content-layer/2

**Files:**

```
flow/engine/commands/render.mjs     # the command
flow/engine/cli.mjs                 # + 'render' in COMMANDS
```

**Test scenarios:** `--list` shows sources/blocks/MISSING; `--check` exit codes (clean, broken, empty); `--write` appends, replaces, no-ops, refuses a missing target and an empty channel, strands no temp file on failure; end-to-end through the real CLI.

**Verification:** `node flow/engine/tests/run.mjs render.test`.

---

### content-layer/4 — The three default rulesets

**Goal:** ship the block library and the default channel composition.

**Requirements:** R7

**Dependencies:** content-layer/2

**Files:**

```
flow/content/vibe.default.json
flow/content/blocks/style/ste100.md
flow/content/blocks/delegation/subagents.md
flow/content/blocks/delegation/workflows.md
vibe.json                           # this repo's own project layer
AGENTS.md                           # the rendered vibe:rules block
```

**Test scenarios:** the shipped tree lints clean; all three ids are composed into their channels; every channel is inside budget; `AGENTS.md`'s block matches the rendered channel.

**Verification:** `node flow/engine/cli.mjs render --check` and the `shipped:` cases in `content.test.mjs`.

---

### content-layer/5 — Wiring: hooks, install, uninstall

**Goal:** the two prompt channels ride the existing hooks; install renders the document channel; uninstall removes it.

**Requirements:** R4, R7

**Dependencies:** content-layer/3, content-layer/4

**Files:**

```
flow/engine/commands/hook.mjs       # inject + doctrine append their channel
install.sh                          # render agents-md after the AGENTS.md merge
flow/scripts/merge-agents.sh        # unmerge strips the vibe:rules block too
```

**Test scenarios:** orders stay first and warns stay last; no content tree ⇒ byte-identical hook output; install writes the block; uninstall removes it and keeps user prose.

**Verification:** `bash flow/tests/adapters/run.sh` plus the `hooks:` cases in `content.test.mjs`.

---

### content-layer/6 — Evidence

**Goal:** the suites that make the above checkable, including the lint's own population floor.

**Requirements:** R5

**Dependencies:** content-layer/1–5

**Files:**

```
flow/engine/tests/content.test.mjs
flow/engine/tests/render.test.mjs
flow/engine/tests/run.mjs           # placeholder-CLI helper knows the new command
flow/engine/tests/cli.test.mjs      # COMMANDS list
```

**Test scenarios:** an empty content tree FAILS `--check`; the shipped tree is asserted non-empty structurally, never by a hand-written count.

**Verification:** `bash tests/run.sh` — all four suites green.
