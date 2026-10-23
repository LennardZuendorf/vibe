---
type: tech
applies:
  - "instruct/**"
  - "vibe.json"
  - "AGENTS.md"
updated: 2026-10-19
---

# Migration

`vibe-instruct migrate` splits today's single Active-Rules block into a
committed block (repo facts, repo-specific lessons) and global blocks
(workflow doctrine, subagent tiers, output style); root `vibe.json` is
read as a one-release alias for `.vibe/instruct.json`, then removed. This
repo's own `merge-agents.sh` and `regen-active-rules.sh` are ported with
file-byte parity as the acceptance test — the migration must reproduce
today's AGENTS.md bytes before it is allowed to diverge.
