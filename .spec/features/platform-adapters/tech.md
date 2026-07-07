---
type: feature-tech
feature: platform-adapters
sibling: product.md
parent: ../../tech.md
updated: 2026-06-06
---

# Feature: Platform Adapters — Architecture

Adapters are thin files that translate runtime-specific affordances into the
platform-neutral `.agents/skills/vibe` core. The Claude Code
adapter goes one step further: it is packaged as an installable **Claude Code
plugin** whose hooks make the flow automatic.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)
**Related:** [../agent-instructions/tech.md](../agent-instructions/tech.md)

---

## Files

```text
# Agent instruction files → agent-instructions feature (AGENTS.md real file;
# CLAUDE.md / WARP.md optional symlinks). This feature owns runtime wiring only.

.claude-plugin/
└── plugin.json                    # Claude Code plugin manifest
.claude/
├── commands/
│   └── flow.md                    # /flow transition command
└── hooks/
    ├── hooks.json                 # event → script wiring
    ├── user-prompt-submit-inject.sh  # inject linked skill's per-state orders (D12)
    ├── pre-tool-use-guard.sh         # allow/warn/block via detect-context.sh
    └── stop-gate.sh                  # end-of-turn exit-predicate smell checks
```

---

## Claude Code plugin

vibe ships as a Claude Code plugin so a single install wires up the command,
the skills, and the hooks against the platform-neutral core.

- **Manifest:** `.claude-plugin/plugin.json` declares name, version, and the
  bundled `commands/`, `skills/`, and `hooks/` (Claude Code discovers `vibe-*`
  and `spec` skills, the `/flow` command, and `hooks/hooks.json`).
- **Hook wiring:** `hooks/hooks.json` maps Claude Code events to the scripts
  below, referencing them via `${CLAUDE_PLUGIN_ROOT}` so the plugin is relocatable.
- **No new state:** the plugin owns no cursor and no spec layout — every hook and
  command reads `.agents/skills/vibe`.

### Hooks (the Stage 2 enforcement layer)

Each hook is a thin shell over `.agents/skills/vibe/scripts/`; the invariant logic lives
once in `detect-context.sh`, never copied into a hook. The inject hook is likewise
thin: it resolves the state's linked skill and injects that skill's orders (D12) —
the orders are authored once, in the skill.

| Hook script | Event | Reads | Behaviour |
|---|---|---|---|
| `user-prompt-submit-inject.sh` | `UserPromptSubmit` | `state.json`, `state-machine.json`, the state's linked `vibe-*` skill | Resolve the current state's linked skill (D12) and inject that skill's per-state **orders block** (no exit codes, no blocking). This is the daily driver — it reminds the agent every turn, so the human is no longer the inject mechanism. Static-content discipline: nothing turn-varying, or the prompt cache rebuilds each turn. Skill-less states (`idle`, `amend`) fall back to the machine's inline string. |
| `pre-tool-use-guard.sh` | `PreToolUse` matcher `Edit\|Write\|NotebookEdit` | `detect-context.sh decide <path>` | Exit 2 on the three hard blocks (`lessons.md` outside compound, root specs outside `strategy.spec`/`feature.compound`, direct `state.json` edits). Warnings (exit 0 + stderr) elsewhere. |
| `stop-gate.sh` | `Stop` | `detect-context.sh` | End-of-turn smell checks: stuck phase (same state N turns), impl touched source without tests, verify entered without review, forgotten `set-state.sh`. Warn-only first; promote individual predicates to blocking only after dogfooding. |

### Earn the teeth

The three enforcement layers are added in order and earn strength through
observation:

1. **Guide** — the inject (Stage 1 behaviour, now automated by the hook).
2. **Guard** — `PreToolUse` hard blocks on the three invariants only.
3. **Gate** — `Stop` predicates, warn-first, blocking only once dogfooding proves
   they are crossed by accident.

Graceful degradation is mandatory (R9): a missing keystone script or unreadable
state exits 0 and never breaks the session.

---

## Adapter Rules

- Read canonical state from `.agents/skills/vibe/state.json`.
- Read canonical transitions from `.agents/skills/vibe/state-machine.json`.
- Invoke the `.agents/skills/vibe` skill for workflow behavior.
- Do not write a platform-specific state cursor.
- Do not introduce platform-specific `.spec/` paths.
- Hooks call the shared decision policy in `detect-context.sh`; they never
  re-implement the allow/warn/block rules.

---

## Install Behavior

The installer should copy core `.agents/**` files, register the Claude Code plugin,
and delegate `AGENTS.md` merge + adapter symlinks to
[agent-instructions](../agent-instructions/tech.md) (`merge-agents.sh`). Never
blindly overwrite user-owned instruction content.
