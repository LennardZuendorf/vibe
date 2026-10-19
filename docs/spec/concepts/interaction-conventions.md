---
type: concept
updated: 2026-10-19
---

# Interaction Conventions

- Use `vibe-flow`, `vibe-spec`, and `vibe-instruct` for their recurring workflows; each is a complete CLI (and, for `flow`, a TUI) on its own — or the `vibe` front door (`vibe spec|flow|instruct …`, v0.5) when all are installed.
- Use `.vibe/run/` for runtime state (cursor, ledger, payloads) and the spec root (`docs/spec/`; legacy `.spec/` still resolves) for durable memory. <!-- literal -->
- In agent-facing prompts, phrase constraints as positive targets first: "write only these paths" before "do not write elsewhere."
- End each phase with a concise receipt: changed files, verification evidence, and next legal transition.
