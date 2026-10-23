---
type: tech
applies:
  - "crates/vibe/**"
  - "vibe/**"
updated: 2026-10-19
---

# Worktree lifecycle

`vibe worktree` — one git worktree per feature/branch, runtime state isolated
per worktree. The v0.4 cursor already resolves per project root, so isolation
needs no new mechanism — each worktree is its own root with its own
gitignored `.vibe/run/`:

- `create <name> [--from <branch>]` — `git worktree add`, then bootstrap
  inside the new worktree: `.vibe/spec.json` pointing at the repo's spec root
  (worktrees share committed memory via git), each installed tool's `init`
  for config and AGENTS.md blocks, `.vibe/run/` seeded empty. Refuses when a
  worktree of that name already exists.
- `list` — worktrees with their cursor (flow/phase/feature), uncommitted-file
  count, days since last ledger event. Staleness is advisory, never a gate.
- `remove <name>` — refuses while the cursor is mid-flow (suggest compound or
  `state set idle`), then removes the worktree and its run dir; deregisters.
  This is the only hard refusal in the lifecycle.
- `archive <name>` — compound-time: moves uncommitted `features/<name>/`
  spec residue to the main checkout, then removes. Committed spec edits
  travel by branch merge; `archive` exists for the residue git does not
  carry.
