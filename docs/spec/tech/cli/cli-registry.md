---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# Repo registry

`~/.config/vibe/repos.json` (`%APPDATA%\vibe\` on Windows):

```json
{ "repos": [ { "path": "/abs/path", "registered": "2026-09-26T00:00:00Z" } ] }
```

Registered by `vibe` at install/init and by `vibe worktree create`;
deregistered by `vibe worktree remove` when a worktree's repo has no other
entry, and by `--uninstall` for the main checkout. The registry is the only
cross-repo discovery mechanism — `vibe space list` never scans the disk.
