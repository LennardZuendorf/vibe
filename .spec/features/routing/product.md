---
type: feature-product
feature: routing
sibling: tech.md
parent: ../../product.md
updated: 2026-05-03
---

# Feature: Routing — Product

The routing layer is invisible to the user but shapes everything they experience. It's what makes shards-code coherent: every component asks the same script the same questions and gets consistent answers.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)

---

## Why this feature exists

Hooks need to know the current phase to decide what to enforce. Commands need to know the current phase to decide what skill to load. CLAUDE.md needs to know the current phase to make sense of the next user prompt. Without a single source of truth, every component re-implements its own state detection — and they drift.

The routing layer solves this with one bash script (`bin/detect-context.sh`) that reads `.spec/.phase` plus filesystem state and returns a JSON object. Hooks call it. Commands call it. The user never invokes it directly.

A small companion script (`bin/set-phase.sh`) is the only sanctioned way to write `.spec/.phase`. Direct edits are hard-blocked at the hook layer.

---

## Requirements

| # | Requirement |
|---|---|
| R1 | Single source of truth: `bin/detect-context.sh` is the only place that reads `.spec/.phase` and returns routing state. |
| R2 | Output is always valid JSON, even on error. Programmer errors (missing `jq`) are the only exception. |
| R3 | Never mutates state. Reads only. State writes go through `bin/set-phase.sh`. |
| R4 | Graceful degradation: missing skills are dropped from the load list with a warning, not an error. Corrupt `.phase` returns `workflow=none` with a warning. |
| R5 | Performance: per-call latency ≤ 50ms, since hooks call it on every user prompt and every Edit/Write. |
| R6 | `bin/set-phase.sh` validates input against the state grammar before writing. Invalid input fails non-zero with a clear stderr message. |
| R7 | State writes are atomic: write to a tmpfile, `mv` into place. Never half-written `.phase`. |
| R8 | `.spec/.phase` itself is gitignored — it's per-developer transient state, not project history. |

---

## What the User Relies On

The user never calls the routing scripts directly, but they depend on the routing layer for these guarantees:

- **Resumability.** Re-running `/code:strategy` or `/code:feature <name>` picks up exactly where the last session left off, because state is on disk and reading it is consistent.
- **Visibility.** SessionStart prints the current phase and skill load list. The user can always see what's active.
- **Suggestions.** Routing tells UserPromptSubmit "this looks like a quick task" or "this is asking for code mid-DESIGN" — without the routing layer, those suggestions would be ad-hoc heuristics scattered across hooks.
- **Graceful failure.** A missing skill or bad state file produces a warning, not a crash. The user can finish their thought and fix the underlying problem afterward.

---

## What the User Will Never See

- A separate routing config file (`.shards/config.json`). Routing is hard-coded in v1.
- A "routing state" UI or status command. `cat .spec/.phase` is the status command.
- Routing-layer errors as session crashes. Errors degrade to warnings.

---

## Non-Goals

- Per-skill availability detection in v1. We trust that if a plugin is installed, its named skills exist. Per-skill detection is v1.1 if a real need surfaces.
- A way to bypass `set-phase.sh`. The whole point of the structural block is that there's no escape hatch — except the script itself.
- Caching routing JSON across hook invocations. `.phase` changes mid-turn (commands write it after each sub-phase). Caching is unsafe.

---

## Open Questions

1. **Per-skill detection.** Today we trust that a present plugin has its named skills. What's the lightest verification — `grep` SKILL.md for the name, or read the plugin manifest? Defer to v1.1.
2. **Stdin prompt parsing.** UserPromptSubmit pipes the prompt in for keyword routing. Where does the keyword list live (hard-coded, a config file)? Default: hard-coded in v1, move to config in v1.1.
3. **Per-project skill overrides.** A project pinning different skills for a phase — that's exactly what `.shards/config.json` is for. v1.1.
