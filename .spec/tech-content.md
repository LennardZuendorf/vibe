---
type: tech-topic
parent: tech.md
scope: content authoring, injection channels, budgets
covers: block format, .vibe override layer, channel triggers, line budgets, authoring lints
updated: 2026-08-10
---

# vibe — Content & Injection

Branch doc for the content layer: how every injected sentence is authored,
composed, and budgeted. Parent: [tech.md](tech.md).

> **Status (2026-08-15).** The authoring/composition/override half has shipped —
> see [features/content-layer/](features/content-layer/tech.md) for what exists.
> Two deltas from the design below, both deliberate: the project layer is a
> single root `vibe.json` (shipped defaults live in
> `flow/content/vibe.default.json`) rather than a `.vibe/compose.json` +
> `policy.json` pair, and `policy.json` — write invariants as data — is NOT
> implemented; `detect-context.sh decide` is still the single source for those.
> Trigger classing (level/edge/event) remains inject-triggers' box; today both
> prompt channels fire on every turn / every session start.

Every injected sentence is authored once, as a **block**, and composed into a
**channel**. One marker grammar, one resolver.

```text
flow/content/
├── policy.json          # write invariants as data — the enforcer reads THIS
├── compose.json         # channel → ordered block ids
└── blocks/**.md         # frontmatter: id, channels, summary; body = long form
.vibe/                   # user layer — never written by install, survives upgrade
├── content/blocks/**.md # override any shipped block by id
└── compose.json         # extend or reorder any channel
```

A block's `summary` renders into terse channels; its body renders into prose
channels. One author point, two verbosities — replacing the previous seven-way
restatement of the write invariants across doctrine, template, root `AGENTS.md`,
enforcer, guard comment, and both READMEs.

`policy.json` is the single source for write invariants: the guard evaluates it
and the renderer prints it. Parity is by construction, retiring the
prose↔code parity test.

### Channels and triggers

`UserPromptSubmit` output persists in the conversation transcript, so payload is
classed by trigger, not emitted uniformly:

| Channel | Trigger | Payload | Budget |
|---|---|---|---|
| `session-start` | session, `compact` | timeless doctrine summaries + user blocks. **No live state** — replayed stale on `--resume`. | ≤15 lines |
| `user-prompt.level` | every turn | current state + its transition command, byte-stable | ≤2 lines |
| `user-prompt.edge` | cursor changed since last inject | full orders, delegate contracts, lessons by state tag | ≤15 lines/transition |
| `user-prompt.event` | only when true | drift nudge, warning drain | 0 normally |
| `verdict` | guard/gate block | rendered from `policy.json` + the offending path; transient, no context cost | — |
| `agents-md` | `init`, `setup.apply` | what the model must *decide* and no hook enforces | ≤40 lines |
| `rules` | matching files touched | path-scoped `.claude/rules/` with `paths:` frontmatter | — |

Edge detection compares the cursor against `.vibe/last-inject`. Authoring rules
are linted by `vibe render`: positive framing with a stated reason, per-channel
line budget, no block text duplicated across channels, and no managed block
present in an ancestor `CLAUDE.md`.
