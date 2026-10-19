---
type: decision
updated: 2026-10-19
---

- **D20 — Contracts are the only coupling.** Versioned schemas + goldens in `contracts/`; tools exchange files under `.vibe/` plus one soft probe call, never imports.
