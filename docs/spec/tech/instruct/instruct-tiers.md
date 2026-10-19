---
type: tech
applies:
  - ".vibe/instruct.json"
  - ".vibe/blocks/**"
  - ".vibe/local/**"
  - "flow/content/**"
  - "instruct/**"
updated: 2026-10-19
---

# Tiers

Four tiers, later wins per block `id`:

| Tier | Path | Rights |
|---|---|---|
| global | `$XDG_CONFIG_HOME/vibe/instruct/` (`~/.config/vibe/instruct/`; Windows `%APPDATA%\vibe\instruct\`) | define/toggle blocks; `session-start` only by default |
| repo-shared | `.vibe/instruct.json` + `.vibe/blocks/*.md` (committed) | define/toggle blocks; **only tier that may compose `agents-md`** |
| repo-local | `.vibe/local/{instruct.json,blocks/}` (git-excluded via `.git/info/exclude`) | define/toggle blocks; never reaches a commit |
| session-runtime | provider payload only | no blocks — text, never persisted |

Cache: `$XDG_CACHE_HOME/vibe/instruct/` (`~/.cache/…`; Windows
`%LOCALAPPDATA%\vibe\instruct\cache\`). A block declares one `id`; a higher
tier with the same `id` replaces, disables, or reorders it — never merges
its text. Rejecting global/local blocks from `agents-md` at lint time is
what lets a clone render byte-identically for every teammate; personal
text must never reach a commit. Repo facts (commands, layout, write
policy, repo-specific lessons) live in the committed block; workflow
doctrine, subagent tiers, and output style live in global blocks;
per-repo personal overrides live in repo-local.
