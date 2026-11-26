---
type: decision
updated: 2026-11-26
---

- **D45 — Superpowers-first feature layer.** Feature design and plans default to `superpowers:brainstorming` → `superpowers:writing-plans` writing their native artifacts under `docs/superpowers/{specs,plans}/`; topic slug == feature name. Fallback to `features/<name>/` only when superpowers is absent, the user says "self", `.vibe/spec.json` sets `"featureLayer": "vibe"`, or OpenSpec interchange is needed. `vibe-spec feature` and `vibe-spec plan` read that key: `"vibe"` selects the fallback folders and `"superpowers"` pins the default; the selected layer answers even when the other holds artifacts, a selected layer with nothing to read is a refusal naming both candidates, and an unknown value is a config error. No layer switch is ever silent. Supersedes "canonical paths beat skill defaults".
