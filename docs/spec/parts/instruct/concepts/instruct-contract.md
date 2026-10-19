---
type: concept
updated: 2026-10-19
---

# Instruct Contract

Four tiers layer over one another — global, repo-shared, repo-local,
session-runtime — later wins per block `id`; only repo-shared may compose
the `agents-md` channel. Channels are trigger-classed and budgeted:

| Channel | Trigger | Budget |
|---|---|---|
| `user-prompt.level` | every turn | ≤2 lines |
| `user-prompt.edge` | cursor advanced since session ledger | ≤15 lines |
| `user-prompt.event` | event occurred | ≤10 lines |
| `session-start` | session, `compact`, `resume` | ≤15 lines |
| `agents-md` | install, `write --check` | ≤80 lines |

Per-turn total (level + edge + event) ≤20 lines. instruct reads provider
payload files and manifests under `.vibe/`; it never learns a peer's tool
name. Full contract, tiers, sources, and lints:
[tech-instruct.md](../../../tech/instruct.md).
