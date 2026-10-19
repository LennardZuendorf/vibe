---
type: decision
updated: 2026-10-19
---

- **D47 — Router + tooth.** The spec skill carries a trigger-only, assertive description; a spec-owned `SessionStart` hook injects a short `using-spec` router; a spec-owned `PreToolUse` hook blocks edits under the spec root and `docs/superpowers/**` unless the matching skill was loaded in the session; it fails open on unreadable input and honours an owner-set `"tooth": "warn"`.
