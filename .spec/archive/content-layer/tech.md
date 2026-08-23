---
type: feature-tech
feature: content-layer
sibling: product.md
parent: ../../tech.md
updated: 2026-08-15
---

# Feature: Content Layer — Architecture

Two engine modules and a data tree. `content.mjs` resolves three layers (shipped
defaults, `.vibe/blocks/**`, `vibe.json`) into channels of blocks, interpolates
typed placeholders, and renders per channel; `commands/render.mjs` is the human
and CI surface (`vibe render`). Hooks call the renderer directly, so the
per-turn and session-start channels ride the injection paths js-core already
established.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
flow/content/vibe.default.json          # shipped channels, budgets, write targets   ~35 LOC
flow/content/blocks/style/ste100.md     # per-turn: brief technical English
flow/content/blocks/delegation/subagents.md   # agents-md: model tiers
flow/content/blocks/delegation/workflows.md   # agents-md: dynamic workflows
flow/engine/content.mjs                 # loader, merge, placeholders, render, lint  ~430 LOC
flow/engine/commands/render.mjs         # vibe render (list/check/channel/--write)   ~170 LOC
flow/engine/blocks.mjs                  # + stripBlock/renderBlock/upsertBlock       ~60 LOC added
flow/engine/commands/hook.mjs           # + two renderChannelSafe() calls
flow/scripts/merge-agents.sh            # unmerge also strips the vibe:rules block
install.sh                              # renders agents-md after the AGENTS.md merge
vibe.json                               # this repo's own project layer (dogfood)
```

---

## Contract / API

```js
// flow/engine/content.mjs
loadContent(root, vibeDir)            // -> {channels, blocks: Map, placeholders, sources, errors}
renderChannel(name, ctx, content?)    // -> {text, blocks, warnings, errors, unresolved}
renderChannelSafe(name, ctx)          // -> string; never throws (the hook path)
checkContent(ctx, content?)           // -> {errors, warnings, blockCount}
// ctx = {root, vibeDir, skillsDir}
```

Config shape (`flow/content/vibe.default.json`, and the same shape in `vibe.json`):

```jsonc
{
  "version": 1,
  "sources": { "lessons": ".spec/lessons.md" },
  "channels": {
    "<name>": {
      "render": "summary" | "body",   // terse prompt form vs prose document form
      "budget": 6,                     // line ceiling; 0 = unbudgeted
      "blocks": ["<id>", "..."],      // REPLACES the shipped list
      "add": ["<id>"], "remove": ["<id>"],  // ...or edits it
      "headings": true,                // default: true for body, false for summary
      "enabled": true,
      "write": { "file": "AGENTS.md", "block": "vibe:rules", "note": "…" }
    }
  },
  "blocks": { "<id>": { "title", "channels", "summary", "body", "file", "enabled" } },
  "placeholders": { "<name>": "<value>" }
}
```

Block file (`flow/content/blocks/**.md`, `.vibe/blocks/**.md` — id defaults to the
path, `a/b.md` → `a.b`):

```markdown
---
id: delegation.subagents
title: Delegating to sub-agents
channels: [agents-md]
---
<!-- vibe:summary -->
terse form for prompt channels
<!-- /vibe:summary -->

prose form for document channels
```

---

## Implementation Detail

**Resolution order.** `<vibeDir>/content/vibe.default.json` and its `blocks/**`
tree load first; `<root>/.vibe/blocks/**` overrides shipped blocks by id;
`<root>/vibe.json` merges last (channel edits, block patches, placeholders).
Later layers never mutate earlier ones, and the shipped layer is replaced
wholesale on upgrade — which is why nothing user-owned lives there.

**Placeholders.** Built-ins resolve from `readCursor` + `loadMachine` (state,
feature, next, writes, reads, delegates, exit), from `runOrders` (`{{orders}}`),
from the doctrine block (`{{doctrine}}`), and from the configured lessons file
(`{{lessons:TAG}}`, matched against `**Tags:**` lines). Built-ins win over
config-supplied names, so a project cannot freeze `{{state}}` to a constant.
Unknown names stay literal and are reported.

**One grammar.** The marker grammar stays owned by `blocks.mjs`, which grew a
write half (`stripBlock`, `renderBlock`, `upsertBlock`) so `content.mjs` and
`render.mjs` never spell a marker — the duplicate-primitive scan enforces this.
`upsertBlock` returns `changed: false` when the rendered content already matches,
which is what makes `--write` idempotent. The lessons path is config data for the
same reason: no engine module may name `.spec` itself.

**Hook wiring.** `runInjectHook` appends the `user-prompt` channel after the
state's orders and before the warnings drain; `runDoctrineHook` appends the
`session-start` channel after the doctrine block. Both go through
`renderChannelSafe`, and a target with no content tree emits byte-identical
output to the pre-content-layer hooks — pinned by a test.

<!-- merge -->
**D15 realized — content is data.** Every injected sentence is a block authored
once (`id` / `title` / `channels`, summary + body), composed per channel by
`vibe render`, with typed placeholders interpolating machine fields. The project
layer is a single root `vibe.json` that install never writes, so a target's
injection policy survives every upgrade. Trigger classing and budget *policy*
(level/edge/event) remain inject-triggers' box; this feature ships the authoring,
composition, override, and lint machinery, plus per-channel line budgets.
<!-- /merge -->
