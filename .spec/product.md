---
type: entrypoint
scope: product
children: []
updated: 2026-05-03
---

# shards-code — Product

A personal Claude Code framework that orchestrates Compound Engineering, Superpowers, and a bundled `/spec` skill into a portable, gently-enforced workflow. It does not reimplement existing skills — it routes to them, encodes preferences as enforcement (hooks) rather than advice (CLAUDE.md alone), and exposes a small surface (three commands) that covers every real workflow without ceremony for trivial work.

**One-liner:** Three commands, one keystone script, gently-enforced workflow — orchestration over reimplementation.

---

## Story

My Claude Code workflow lives in my head and in scattered CLAUDE.md fragments. It's not portable across machines, not enforced consistently, and assumes I'll remember to invoke the right skills at the right time. The existing ecosystem provides excellent building blocks — Compound Engineering's planning and review agents, Superpowers' brainstorming and TDD discipline, my own `/spec` skill for spec hygiene — but no one orchestrates them into a single coherent loop. Every project re-invents the lifecycle.

shards-code is the thin orchestration layer. Three commands, one keystone routing script, four gentle hooks, gently enforced — and the existing ecosystem keeps doing what it's good at.

---

## Requirements (Mini PRD)

At a project level, shards-code must:

1. **Cover every workflow with at most three commands.** Quick fixes, project bootstrap, full feature lifecycle. No fourth command in v1.
2. **Make the current state visible.** At any moment the user (and Claude) can see which workflow is active, which phase, and which feature.
3. **Route consistently from one source.** Hooks and commands ask the same script (`bin/detect-context.sh`) what to do. No parallel routing logic.
4. **Enforce gently.** Warnings on stderr by default. Hard blocks reserved for invariants whose violation would corrupt downstream tools.
5. **Degrade gracefully.** Missing skill, corrupt state file, missing `.spec/` — emit a warning, fall back to neutral, never crash a session.
6. **Stay portable.** No build step. No CLI binary. No new runtime dependencies beyond what `/spec` already needs (bash, jq).
7. **Be resumable.** State lives on disk in `.spec/.phase`. A session that ends mid-feature picks up exactly where it left off.

---

## Design Principles

1. **Orchestrate, don't reimplement.** Existing plugins are maintained by their authors. Routing to them means we get their updates for free and don't fork their bugs.
2. **One source of routing truth.** A single keystone script determines current state and what to load. Hooks and commands both call it. No second source.
3. **Gentle enforcement first.** Warnings to stderr are the default. Hard blocks are reserved for two known footguns. Adding more blocks requires evidence of repeated misuse.
4. **Three commands cover everything.** Quick, strategy, feature. If something doesn't fit, it's either a quick task in disguise or a feature in disguise.
5. **State is one line.** `.spec/.phase` is `<workflow>:<phase>[:<feature>]`. No JSON, no multiple files, no hidden state.
6. **Specs are the backbone.** The `/spec` skill is bundled and always required. Every command reads or writes specs at known points.
7. **Portable, not packaged.** Markdown plus bash plus Claude Code primitives. `install.sh` symlinks files into `~/.claude/` and the project.

---

## Target User

Me. One person. Personal config made portable, not a framework distributed to others. Decisions favor my workflow, not generality. If someone else adopts it, they should expect to fork.

---

## The Three Commands (Summary)

| Command | When | Outputs |
|---|---|---|
| **`/code:quick <task>`** | Ad-hoc fix, ≤5 files, no architecture change | Source + ephemeral `.spec/.quick-plan.md` |
| **`/code:strategy`** | Project bootstrap or major refocus | Global specs (`product.md`, `tech.md`, optional design + plan) |
| **`/code:feature <name>`** | Build a real feature end-to-end | Feature specs → source → global spec deltas (only on COMPOUND) |

Detailed UX and behavior live in [features/commands/product.md](features/commands/product.md).

---

## External Dependencies

shards-code orchestrates three external systems. It does not reimplement them.

| System | Surface | Used in |
|---|---|---|
| **Compound Engineering** | `/ce-strategy`, `/ce-ideate`, `/ce-plan`, `/ce-deepen-plan`, `/ce-code-review`, `/ce-commit-push-pr`, `/ce-compound`, `/ce-work` | strategy DESIGN, feature DESIGN/REVIEW/SHIP/COMPOUND |
| **Superpowers** | `superpowers:brainstorming`, `:test-driven-development`, `:subagent-driven-development`, `:systematic-debugging` | DISCUSS phases, IMPL:WORK, debugging fallback |
| **Bundled `/spec` skill** | `/spec`, `setup.sh`, `validate.sh` | every SPEC phase, validation gates |

If a skill is missing at runtime, the routing layer drops it from the load list and emits a stderr warning. Workflows degrade, they don't fail.

---

## Implementation Phases

| Phase | Goal |
|---|---|
| **M1: Foundation** | Routing keystone + state writer + first command + visibility hook |
| **M2: Enforcement** | Gentle phase gate (warnings + 2 hard blocks) + strategy command |
| **M3: Lifecycle** | Feature command + merge tooling + remaining hooks + installer |
| **v1.1** | `/code:amend` + `.shards/config.json` provider overrides (deferred) |

Detailed milestone breakdown in [plan.md](plan.md). Per-feature requirements and architecture in [features/](features/).

---

## Non-Goals

- **Not a framework for others.** Personal config made portable. Forks expected.
- **Not a CLI tool.** No binaries, no npm. Markdown + bash + Claude Code primitives.
- **Not strict-by-default.** Hard blocks are two and only two in v1.
- **Not a replacement for any plugin.** CE, Superpowers, `/spec` continue to work standalone.
- **Not project-specific.** Lives in `~/.claude/` plus a per-project `.spec/`.
- **Not a marketplace.** No plugin discovery, no user-installable extensions. The three external dependencies are pinned at the source-name level.

---

## Open Questions

Project-level decisions still open. Feature-level questions live with each feature.

1. **Quick mode plan threshold.** Currently "any logic change → plan, ≤2 lines literal text → direct." Stricter (always plan), default, or looser (only architecture changes plan)?
2. **Hard-block list.** v1 has two. Are there other footguns once we run real workflows?
3. **`.shards/config.json` for provider overrides.** v1 or v1.1?
4. **`/code:amend`.** v1 or v1.1?
5. **Skill auto-loading.** Stderr suggestions only, or actual skill activation when hooks detect a missing skill in the load list?

Defaults if forced: stricter quick threshold, keep blocks minimal, defer config to v1.1, defer amend to v1.1, stderr-only first.

---

## Features

Each feature has its own product (requirements) and tech (architecture) spec under `.spec/features/<name>/`. Features are short-lived: created during DESIGN, consumed during IMPL, merged into globals during COMPOUND, archived after.

| Feature | Covers |
|---|---|
| **[features/commands/](features/commands/product.md)** | The three commands and their lifecycles. UX, sub-phases, gates, escape hatches. The user-facing surface. |
| **[features/routing/](features/routing/product.md)** | The keystone routing layer: `bin/detect-context.sh` + `bin/set-phase.sh`. State management, JSON contract, validation. |
| **[features/hooks/](features/hooks/product.md)** | The four hooks: SessionStart, UserPromptSubmit, PreToolUse, Stop. Warnings, the two hard blocks, performance contract. |
