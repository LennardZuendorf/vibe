---
type: tech
applies:
  - ".vibe/instruct.json"
  - ".vibe/blocks/**"
  - ".vibe/local/**"
  - "flow/content/**"
  - "instruct/**"
updated: 2026-10-19
---

# Budgets and channels

| Channel | Trigger | Budget |
|---|---|---|
| `user-prompt.level` | every turn | ≤2 lines |
| `user-prompt.edge` | cursor advanced since session ledger | ≤15 lines |
| `user-prompt.event` | event occurred | ≤10 lines |
| `session-start` | session, `compact`, `resume` | ≤15 lines |
| `agents-md` | install, `write --check` | ≤80 lines |

Per-turn total (level + edge + event) ≤20 lines. A render that cannot meet
its channel's budget is a lint failure, not a runtime truncation of
intent — truncation (`[truncated: run vibe-flow orders]`) only ever trims
a provider's own overflow, per [tech-contracts.md](../contracts.md).
Global-tier blocks default to `session-start`; entering
`user-prompt.level` is explicit opt-in per block, because that channel is
the one budget with zero slack.

## Block format

Carried from the former content layer unchanged in shape:

```text
.vibe/blocks/<id>.md
---
id: <block-id>
title: <short title>
channels: [session-start, user-prompt.level]
summary: <renders into terse channels>
---
<body — renders into prose channels (agents-md)>
```

One author point, two verbosities: `summary` for terse channels, body for
`agents-md`. Marker grammar (owner `instruct`, order 30) is defined once,
in `vibe-core` — see [tech-contracts.md](../contracts.md).
