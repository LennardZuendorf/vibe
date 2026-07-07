---
type: feature-product
feature: platform-adapters
sibling: tech.md
parent: ../../product.md
updated: 2026-06-06
---

# Feature: Platform Adapters — Product

Platform adapters expose the same `vibe` flow to Codex, Claude Code, and future
agent runtimes without making any one platform canonical. For Claude Code
specifically, the adapter ships as an **installable Claude Code plugin** that
bundles the commands, skills, and — crucially — the **hooks** that make the flow
automatic and guard its invariants.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)
**Related:** [../agent-instructions/product.md](../agent-instructions/product.md),
[../vibe-flow/product.md](../vibe-flow/product.md)

---

## Scope

| Owns | Does not own |
|---|---|
| Claude Code plugin (`.claude-plugin/`), hooks, `/flow` command | `AGENTS.md` template merge and adapter symlinks (→ [agent-instructions](../agent-instructions/product.md)) |
| `install.sh` — copy core `.agents/**`, register plugin | State machine and `vibe-*` skill bodies (→ [vibe-flow](../vibe-flow/product.md)) |
| Runtime-specific hook wiring (`UserPromptSubmit`, `PreToolUse`, `Stop`) | `.spec/` document format (→ root [tech.md](../../tech.md) Spec Framework Contract) |

---

## Why this feature exists

Codex and Claude Code read different instruction files and support different
integration points. The workflow should still behave the same because both
adapters point at `.agents/skills/vibe`.

Claude Code, unlike a plain `AGENTS.md` reader, supports **hooks** and a
**plugin** packaging format. That is a first-class part of building the flow, not
an afterthought: the hooks are what turn the flow from "guidance the agent may
follow" into "guidance that fires every turn, with the core invariants guarded."
So part of shipping vibe is building a Claude Code plugin that installs the
core plus its hooks in one step.

---

## The Claude Code plugin

vibe ships a Claude Code plugin (`.claude-plugin/plugin.json` + bundled
`commands/`, `skills/`, and `hooks/hooks.json`). Installing the plugin wires up:

- the `/flow` command and the `vibe-*` skills,
- the three **hooks** below, configured against `.agents/skills/vibe`.

The hooks are the **Stage 2 enforcement layer** (Stage 1 is guidance-only). They
are earned, not assumed: shipped warn-first, promoted to blocking only after
dogfooding (see [../../plan.md](../../plan.md) and the "earn the teeth" principle).

| Hook | Claude Code event | Role | Strength |
|---|---|---|---|
| **Inject** | `UserPromptSubmit` | Pull the current state's orders from its linked `vibe-*` skill and inject them every turn (D12 — the daily driver, removes the human as the inject mechanism). | Guidance |
| **Guard** | `PreToolUse` (`Edit\|Write\|NotebookEdit`) | Call the shared allow/warn/block decision policy; hard-block the three invariants, warn elsewhere. | Deterministic |
| **Gate** | `Stop` | End-of-turn exit-predicate smell checks (stuck phase, impl without tests, forgotten `set-state.sh`). | Warn first |

All three are thin shells over `.agents/skills/vibe/scripts/` — the inject reads
`state-machine.json` to resolve the state's linked skill and injects that skill's
orders (D12); the guard and gate call `detect-context.sh`. No invariant logic or
order text is duplicated in the hooks.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | `AGENTS.md` mirrors the core workflow for Codex-style agents. Provisioning and merge during init live in [agent-instructions](../agent-instructions/product.md). |
| R2 | `CLAUDE.md` mirrors the core workflow for Claude Code (typically a symlink to `AGENTS.md`; see agent-instructions). |
| R3 | Claude slash commands and hooks read `.agents/skills/vibe`, not `.claude/state.json`. |
| R4 | Adapter files do not define a separate spec layout or state model. |
| R5 | Installation preserves existing project instructions and offers diffs when merging. |
| R6 | vibe is installable as a Claude Code plugin (`.claude-plugin/plugin.json`) that bundles the `/flow` command, the `vibe-*` skills, and the hooks. |
| R7 | The plugin includes three hooks — `UserPromptSubmit` inject, `PreToolUse` guard, `Stop` gate — wired to `.agents/skills/vibe`. |
| R8 | Hooks are thin shells over `.agents/skills/vibe/scripts/`; the allow/warn/block policy lives once in `detect-context.sh`, never duplicated per hook. |
| R9 | Hooks degrade gracefully: a missing script or unreadable state exits 0 (never breaks the session); blocks are earned warn-first. |

---

## Outputs

- `.claude-plugin/plugin.json` (plugin manifest)
- `.claude/commands/flow.md`
- `.claude/hooks/hooks.json` + the three hook scripts
  (`user-prompt-submit-inject.sh`, `pre-tool-use-guard.sh`, `stop-gate.sh`)
- installer/setup behavior (incl. plugin install path)

---

## Non-Goals

- `AGENTS.md` / `CLAUDE.md` content provisioning — [agent-instructions](../agent-instructions/product.md)
- Duplicating the state machine per platform
- Making `.claude/` canonical
- Making Codex-specific desktop behavior part of the core flow
- Re-implementing invariant logic inside hooks instead of calling the shared
  decision policy
- Shipping blocking hooks before dogfooding earns them

