---
type: tech-topic
parent: tech.md
scope: content authoring, injection channels, budgets
covers: block format, .vibe override layer, channel triggers, line budgets, authoring lints
updated: 2026-08-22
---

# vibe — Content & Injection

Branch doc for the content layer: how every injected sentence is authored,
composed, and budgeted. Parent: [tech.md](tech.md).

Every injected sentence is authored once, as a **block**, and composed into a
**channel**. One marker grammar, one resolver.

```text
flow/content/
├── policy.json          # write invariants as data — the enforcer reads THIS
├── vibe.default.json    # shipped channels + block metadata
└── blocks/**.md         # frontmatter: id, channels, summary; body = long form
vibe.json                # the project layer, at the repo root — never written by
                         # install, survives every upgrade; merges OVER the
                         # shipped defaults (add / remove / replace per channel,
                         # and define blocks of your own)
.vibe/last-inject        # gitignored runtime marker for edge detection
```

Every path `vibe.json` names — a block's `file`, `sources.*`, a channel's
`write.file` — is confined to the repo before it is read or written. The config
is repository content, so an unconfined path would let a cloned repo inject an
arbitrary file into every turn.

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
| `user-prompt.edge` | cursor changed since last inject | full orders (`{{orders}}` is required — `render --check` errors without it), delegate contracts | ≤15 lines |
| `user-prompt.event` | only when true | drift nudge, warning drain (≤10 lines, duplicates collapsed to `<line> (xN)`) | 0 normally |
| `user-prompt` | compatibility alias for the three above | — | ≤6 lines |
| `agents-md` | install, `setup.apply`, `render agents-md --write` | what the model must *decide* and no hook enforces | ≤80 lines |

Planned, not shipped: a `verdict` channel rendering the guard's block message
from `policy.json`, and a `rules` channel for path-scoped `.claude/rules/`
fragments.

Edge detection compares the cursor against `.vibe/last-inject`. Authoring rules
are linted by `vibe render`: positive framing with a stated reason, per-channel
line budget, no block text duplicated across channels, and no managed block
present in an ancestor `CLAUDE.md`.
