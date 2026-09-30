---
type: tech-topic
parent: tech.md
scope: instruction tiers, budgets, sources
covers: tiers, budgets and channels, block format, lints, sources and sync, provider execution, platform file targets, rewrite triggers, commands, migration
updated: 2026-09-04
---

# Instruct — Tech

[tech.md](tech.md)

Branch doc for the instruct contract: how every injected sentence is
authored, layered across tiers, composed into channels, and budgeted.
Absorbs the former `tech-content.md`. **Shipped today:** `flow/content/`
(`policy.json`, `vibe.default.json`, `blocks/**.md`) plus a root `vibe.json`
override, rendered by the Node engine — one shipped/repo split, no global
tier, no sources, no providers. This doc describes the v0.4 target.

## Tiers

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

## Budgets and channels

| Channel | Trigger | Budget |
|---|---|---|
| `user-prompt.level` | every turn | ≤2 lines |
| `user-prompt.edge` | cursor advanced since session ledger | ≤15 lines |
| `user-prompt.event` | event occurred | ≤10 lines |
| `session-start` | session, `compact`, `resume` | ≤15 lines |
| `agents-md` | install, `write --check` | ≤80 lines |

Per-turn total (level + edge + event) ≤20 lines. A render that cannot meet
its channel's budget is a lint failure, not a runtime truncation of
intent — truncation (`[truncated: run vibe-flow orders]`) only ever trims
a provider's own overflow, per [tech-contracts.md](tech-contracts.md).
Global-tier blocks default to `session-start`; entering
`user-prompt.level` is explicit opt-in per block, because that channel is
the one budget with zero slack.

## Block format

Carried from the former content layer unchanged in shape:

```text
.vibe/blocks/<id>.md
---
id: <block-id>
title: <short title>
channels: [session-start, user-prompt.level]
summary: <renders into terse channels>
---
<body — renders into prose channels (agents-md)>
```

One author point, two verbosities: `summary` for terse channels, body for
`agents-md`. Marker grammar (owner `instruct`, order 30) is defined once,
in `vibe-core` — see [tech-contracts.md](tech-contracts.md).

## Lints

`vibe-instruct lint` fails a render on: a channel budget exceeded; a
block's justification framed negatively without a stated reason ("don't
X" without why); the same text duplicated across two channels; a
repo-shared block declaring a `paths:`-scoped rule with no matching
`.claude/rules/` target; a managed block found in an ancestor AGENTS.md or
CLAUDE.md (must refuse, never overwrite); a block declaring
`enforced-by:` for a channel where the named mechanism does not exist.
One fixture per rule; a clean fixture tree passes with zero findings.

## Sources and sync

```json
{"type": "dir|git|https", "path|url": "...", "ref": "...", "commit": "...",
 "sha256": "...", "ttl": "24h", "scope": "global|repo"}
```

Repo-scope sources must pin (`commit` or `sha256`) or the loader refuses —
a clone must never fetch floating content. Global sources may float on
`ref`. `vibe-instruct sync`: fetch, verify pins, update
`instruct.lock.json`, rewrite generated files (below); offline keeps the
cache, prints one warning, exits 0. Session start does no network and no
subprocess, ≤200 ms wall: it compares the lock and cache mtimes against
each source's TTL and prints one line —
`instruct: 2 sources stale — run vibe-instruct sync` — or nothing. Fetched
content is data: only `blocks/*.md` and channel toggles are read from a
fetched `instruct.json`; providers, hook definitions, and any path
outside the source's own cache directory are ignored, with a warning.

## Provider execution

instruct reads payload files under `.vibe/run/inject/` and manifests
under `.vibe/providers/` — see [tech-contracts.md](tech-contracts.md) for
the schemas. From instruct's side: providers run after instruct's own
blocks, in manifest order; a provider never learns another tool's
internals, only its declared channel/trigger/budget. instruct owns the
session ledger (`.vibe/run/instruct/sessions/<id>.json`) that decides
which `seq` and which event `id`s have already been shown.

## File targets and rewrite triggers

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

## Migration

`vibe-instruct migrate` splits today's single Active-Rules block into a
committed block (repo facts, repo-specific lessons) and global blocks
(workflow doctrine, subagent tiers, output style); root `vibe.json` is
read as a one-release alias for `.vibe/instruct.json`, then removed. This
repo's own `merge-agents.sh` and `regen-active-rules.sh` are ported with
file-byte parity as the acceptance test — the migration must reproduce
today's AGENTS.md bytes before it is allowed to diverge.
