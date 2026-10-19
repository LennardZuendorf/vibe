---
type: decision
updated: 2026-10-19
---

- **D45 — Superpowers-first feature layer.** Feature design and plans default to `superpowers:brainstorming` → `superpowers:writing-plans` writing their native artifacts under `docs/superpowers/{specs,plans}/`; topic slug == feature name. Fallback to `features/<name>/` only when superpowers is absent, the user says "self", `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is needed. Supersedes "canonical paths beat skill defaults".
