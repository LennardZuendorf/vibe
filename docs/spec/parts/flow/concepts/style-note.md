---
type: concept
updated: 2026-10-19
---

# Style

Output density is governed by one machine-level `style` note carried in `flow`'s `state-machine.json`, not a per-state level: no filler or hedging, compress receipts and subagent→orchestrator summaries. Compression is **output only** — it never reduces reasoning depth, and code, paths, and commands stay byte-exact. Regardless of density, security warnings and irreversible-action confirmations stay in full prose. `instruct` composes and injects the same note every turn so adapters and subagents stay consistent (see [tech-instruct.md](../../../tech/instruct.md)).
