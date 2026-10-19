---
type: decision
updated: 2026-10-19
---

- **D28 — Ledger + advisory signals.** Append-only `ledger.jsonl`; `vibe-signals` cannot be depended on by core, so guard and gate structurally cannot read it.
