---
type: decision
updated: 2026-10-19
---

- **D33 — Spec root and discovery.** Default `docs/spec/`, legacy `.spec/`; `--root` → env → config → default → parent search (stops at the directory holding `.git`) → exit 3. `docs/superpowers/` is a recognised artifact root: vibe reads it and deletes from it only at compound (D46). <!-- literal -->
