---
type: feature-tech
feature: inject-triggers
sibling: product.md
parent: ../../tech.md
updated: 2026-08-15
---

# Feature: Inject Triggers — Architecture

Three additions to the content layer, one subtraction from the hooks. Channels
gain a `trigger` field and an edge detector backed by `.vibe/last-inject`; the
write invariants move into `flow/content/policy.json` behind a `vibe policy`
command that `detect-context.sh` delegates to; and the payload corrections
(no cursor at session start, bounded relay, no predicate 3) land in `hook.mjs`.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
flow/content/policy.json                # write invariants as data              ~40 LOC
flow/engine/policy.mjs                  # load + decide(path, state)            ~120 LOC
flow/engine/commands/policy.mjs         # vibe policy decide|list|render        ~90 LOC
flow/engine/content.mjs                 # + trigger classing, edge detection    ~80 LOC added
flow/engine/commands/hook.mjs           # + trigger-aware inject, relay bounds  ~60 LOC changed
flow/content/vibe.default.json          # + the three user-prompt.* channels
flow/content/blocks/flow/level.md       # {{state}} + {{transition}}
flow/content/blocks/flow/edge.md        # {{orders}} + {{delegates}} + {{lessons:…}}
flow/content/blocks/flow/invariants.md  # {{invariants}} — rendered from policy.json
flow/scripts/detect-context.sh          # decide delegates to the engine, bash fallback kept
flow/SKILL.md                           # doctrine block sources its invariants from the render
```

---

## Contract / API

```js
// flow/engine/policy.mjs
loadPolicy(vibeDir)                  // -> {rules: [{id, match, states, verdict, reason}], errors}
decide(policy, relPath, state)       // -> {verdict: 'allow'|'warn'|'block', reason, ruleId}
renderInvariants(policy)             // -> prose lines for {{invariants}}

// flow/engine/content.mjs (added)
channelTrigger(channel)              // -> 'level' | 'edge' | 'event'
cursorChangedSince(root, cursorKey)  // -> boolean   (.vibe/last-inject)
recordInject(root, cursorKey)        // writes .vibe/last-inject atomically
```

`policy.json` rule shape:

```jsonc
{
  "version": 1,
  "rules": [
    {
      "id": "lessons",
      "match": ".spec/lessons.md",              // exact path or a glob
      "states": ["feature.compound", "setup.apply", "strategy.spec", "quick.verify"],
      "verdict": "block",                        // verdict OUTSIDE the listed states
      "reason": "lessons are written at flow end; append during compound"
    }
  ]
}
```

Channel config gains one key, defaulting to `level` for prompt channels and
`event` for the drift/warn surface:

```jsonc
"user-prompt.edge": { "trigger": "edge", "render": "summary", "budget": 15, "blocks": ["flow.edge"] }
```

---

## Implementation Detail

**Edge detection.** `.vibe/last-inject` holds the cursor key the previous inject
saw. The inject hook compares, emits the edge channel when they differ, then
records the new key. A missing file means "everything is an edge" — the
fail-open direction, because a first inject after install must carry the full
orders. The file is gitignored runtime state, like the cursor.

**Policy delegation.** `detect-context.sh decide <path>` keeps its exit-code
contract byte-for-byte. With `node` present it delegates to `vibe policy decide`;
without it, its existing bash branch answers. A differential test drives a matrix
of every guarded path against all 13 states through both paths and asserts equal
verdicts — the same oracle discipline js-core used for the ported commands.

**Prose from policy.** `{{invariants}}` renders the rules as the sentence the
doctrine block and the `AGENTS.md` instructions carry today. That retires the
prose↔code parity test in `flow/tests/run.sh`: the text is generated, so it
cannot drift. Removing that test is part of the unit that replaces it — never
before.

**Subtractions.** The `SessionStart` cursor line is deleted (stale on resume; the
level channel covers it every turn). Predicate 3 is deleted from the stop gate
for the same reason. The relay drains at most 10 lines per turn, collapsing
duplicates to `<line> (xN)`.

<!-- merge -->
**D16 realized — injection is trigger-classed and budgeted.** Prompt payload is
split level / edge / event: two byte-stable lines every turn, the full orders
only on a cursor change (tracked in `.vibe/last-inject`), and event text only
when an event occurred. **D15 completed** — `policy.json` makes the write
invariants data, read by the enforcer and rendered into every prose surface, so
parity is by construction rather than by test.
<!-- /merge -->
