---
type: tech
applies:
  - "instruct/**"
  - "AGENTS.md"
  - "CLAUDE.md"
  - ".claude/rules/**"
  - ".opencode/**"
updated: 2026-10-19
---

# File targets and rewrite triggers

| Platform | Session start | Per turn | Other |
|---|---|---|---|
| Claude Code | `hook session-start` — tiers + one staleness/drift line | `hook prompt` — level/edge/event | `@AGENTS.md` created by `init` if absent (existing symlink left alone); `write --claude-local` emits `CLAUDE.local.md` on request only; `.claude/rules/vibe-instruct-<id>.md` generated for repo-shared blocks declaring `paths:` |
| OpenCode | `system.transform` pushes `session-start` once; `session.compacting` re-pushes it | `chat.message` appends a text part | `config` hook appends `.vibe/local/generated/instructions.md` to `instructions`; `write` also maintains a block in the global `~/.config/opencode/AGENTS.md` |
| Codex / other | committed `agents-md` block only | — | — |

Rewrite triggers for generated files and the managed block: `sync`,
explicit `write`, and flow's `compound` invoking `write` — **never**
session start, which only checks and warns.

## Commands (v1)

`init`, `render <channel>`, `write [--check|--claude-local|--rules|--opencode-global]`,
`sync`, `sources {list,add,rm}`, `providers {list,run}`, `blocks {list,explain <id>}`,
`hook {session-start,prompt}`, `lint`, `doctor`.
