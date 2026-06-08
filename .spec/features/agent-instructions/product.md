---
type: feature-product
feature: agent-instructions
sibling: tech.md
parent: ../../product.md
updated: 2026-06-06
---

# Feature: Agent Instructions — Product

Bootstrap and repair the repo's **agent instruction file** during vibe init:
ship a canonical `AGENTS.md` template from the `vibe-setup` skill, merge it into
the target repo without clobbering user content, and optionally create
platform-specific **symlink adapters** (`CLAUDE.md`, `WARP.md`, …) that point at
`AGENTS.md`.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)
**Related:** [../platform-adapters/product.md](../platform-adapters/product.md),
[../vibe-flow/product.md](../vibe-flow/product.md)

---

## Scope

| Owns | Does not own |
|---|---|
| `AGENTS.md` template, marker-aware merge during `vibe-setup` | Flow state machine, `vibe-*` skill bodies (→ [vibe-flow](../vibe-flow/product.md)) |
| Optional adapter symlinks (`CLAUDE.md`, `WARP.md`, …) | Claude plugin, hooks, `/flow` (→ [platform-adapters](../platform-adapters/product.md)) |
| `vibe:instructions` managed block; preserves `vibe:active-rules` | `.spec/` document format (→ root [tech.md](../../tech.md) Spec Framework Contract) |

---

## Why this feature exists

Every agent runtime reads a different instruction filename, but the workflow
content must stay identical. Today `vibe-setup` still merges a small
**constitution block** into `CLAUDE.md` and `AGENTS.md` separately — duplicative,
easy to drift, and out of step with the new full **engineering guide** model
(the repo's `AGENTS.md` is now the single canonical instruction set).

This feature makes init behave like `spec setup`: pull a template from skill
references, merge into the local file, and let the user opt into symlinks for
runtimes that expect another name. One source of truth, many adapter filenames.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | `vibe-setup` ships a canonical `AGENTS.md` template under its skill `reference/templates/` directory. |
| R2 | `setup.apply` merges the template into the repo-root `AGENTS.md` **inside managed markers only**; content outside markers is user-owned and MUST NOT be overwritten. |
| R3 | If `AGENTS.md` is missing, `setup.apply` creates it from the template (including the empty `active-rules` managed block). |
| R4 | `setup.apply` offers optional **adapter symlinks** the user may enable: at minimum `CLAUDE.md` and `WARP.md`, both pointing at `AGENTS.md`. Additional adapters MAY be added later without changing the core contract. |
| R5 | Symlink creation MUST NOT replace an existing **real file** without showing a diff and getting explicit confirmation. |
| R6 | If `CLAUDE.md` (or another adapter) already symlinks `AGENTS.md`, `setup.apply` MUST skip it. |
| R7 | `setup.detect` reports: `AGENTS.md` present/missing, managed block present/stale, each known adapter (present / symlink / real file / absent). |
| R8 | The template and merge logic MUST preserve the `<!-- vibe:active-rules:start/end -->` block; `regen-active-rules.sh` continues to own that digest. |
| R9 | `setup.apply` deprecates the old **constitution block** merge path — constitution content lives in the `AGENTS.md` template's managed section instead. |
| R10 | Adapter choice is **user-driven** at init (prompt, flag, or config); vibe MUST NOT silently create symlinks the user did not request. |

---

## User Experience

### Fresh repo (no `AGENTS.md`)

```
User: set up vibe

Agent (setup.detect): AGENTS.md missing. CLAUDE.md absent. WARP.md absent.
Agent (setup.apply):  Created AGENTS.md from template.
                      Create adapter symlinks? [CLAUDE.md, WARP.md, none]
User: CLAUDE.md
Agent:              ln -s AGENTS.md CLAUDE.md. Done.
```

### Existing repo (user owns preamble)

```
AGENTS.md exists with custom "## Our Team" section above the managed block.

setup.apply: merges only inside <!-- vibe:instructions:start/end -->.
             reports "preserved user preamble (12 lines)".
             asks about symlinks only for adapters not already correct.
```

### Drifted managed block

```
setup.detect: managed block present but differs from template (v1.2 → v1.3).
setup.apply:  shows diff inside markers; asks before replacing.
```

---

## Adapter catalogue (initial)

| Adapter file | Runtime | Default |
|---|---|---|
| `AGENTS.md` | Codex, Cursor, Copilot, Warp, Gemini CLI, … | **always** (real file) |
| `CLAUDE.md` | Claude Code | opt-in symlink |
| `WARP.md` | Warp (legacy name; `AGENTS.md` preferred by Warp since 2025) | opt-in symlink |

Warp reads `AGENTS.md` natively — a `WARP.md` symlink is only for teams still
on the legacy filename. If both `WARP.md` and `AGENTS.md` exist as real files,
Warp prefers `WARP.md`; symlinking `WARP.md` → `AGENTS.md` avoids that conflict.

Future adapters (`.cursorrules`, `GEMINI.md`, …) are out of scope for v1 unless
trivial to add to the same manifest.

---

## Outputs

- `.agents/skills/vibe-setup/reference/templates/AGENTS.md` (canonical template)
- Updated `vibe-setup` skill (`setup.detect` / `setup.apply` steps)
- Optional `install-adapters.sh` or inline logic in `setup.apply`
- Repo-root `AGENTS.md` (merged) and user-selected adapter symlinks

---

## Non-Goals

- Per-platform instruction **content** (hooks, plugin manifest, `.claude/*`) —
  stays in [platform-adapters](../platform-adapters/product.md)
- Auto-detecting which IDE the user has installed and silently symlinking
- Nested/monorepo `AGENTS.md` files (root only for v1)
- Replacing Warp's `/init` or Claude's `@import` — vibe only provisions files

---

## Decided

1. **Managed marker name:** `vibe:instructions` (replaces `vibe:constitution`). One-time
   migration from constitution markers when detected. Dogfood repo needs **wrap
   migration** — see [plan.md](plan.md) AI0.
2. **Non-interactive mode** — defer `--adapters` flag unless `install.sh` (U6) needs it.

## Open Questions

1. **Non-interactive mode** — should `setup.apply` accept `--adapters claude,warp` or read
   `.vibe/adapters.json` for CI/scripted init? Defer unless needed for platform-adapters U6.
