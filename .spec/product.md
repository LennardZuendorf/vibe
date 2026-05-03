---
type: entrypoint
scope: product
children:
  - product-commands.md
updated: 2026-05-03
---

# shards-code — Product

A personal Claude Code framework that orchestrates Compound Engineering, Superpowers, and a bundled `/spec` skill into a portable, gently-enforced workflow. It does not reimplement existing skills — it routes to them, encodes preferences as enforcement (hooks) rather than advice (CLAUDE.md alone), and exposes a small surface (three commands) that covers every real workflow without ceremony for trivial work.

**One-liner:** Three commands, one keystone script, gently-enforced workflow — orchestration over reimplementation.

---

## The Problem

My Claude Code workflow lives in my head and in scattered CLAUDE.md fragments. It's not portable across machines, not enforced consistently, and assumes I'll remember to invoke the right skills at the right time. The existing ecosystem provides excellent building blocks — Compound Engineering's planning and review agents, Superpowers' brainstorming and TDD discipline, my own `/spec` skill for spec hygiene — but no one orchestrates them into a single coherent loop. Every project re-invents the lifecycle.

## The Solution

shards-code is a thin, opinionated orchestration layer:

1. **Three commands cover everything.** `/code:quick` for ad-hoc fixes, `/code:strategy` for project bootstrap or refocus, `/code:feature` for the full lifecycle of a real feature. Anything more granular is a rabbit hole.
2. **One keystone script routes everything.** `bin/detect-context.sh` reads `.spec/.phase` and project state, returns JSON describing the current workflow, phase, skills to load, paths to read, and warnings. Hooks call it. Commands call it. Nothing else makes routing decisions.
3. **Hooks gently enforce.** Four hooks (SessionStart, UserPromptSubmit, PreToolUse, Stop) emit warnings to stderr. Only two structural footguns are hard-blocked: writing `lessons.md` outside `IMPL:COMPOUND`, and writing global specs outside `IMPL:COMPOUND`. Plus direct edits to `.spec/.phase` (must go through `bin/set-phase.sh`).
4. **State is one line.** `.spec/.phase` holds `<workflow>:<phase>[:<feature>]`. Readable, resumable, scriptable.

---

## Design Principles

1. **Orchestrate, don't reimplement.** Existing plugins are maintained by their authors. Routing to them means we get their updates for free and don't fork their bugs.
2. **Detect-context is the keystone.** A single script determines current state and what to load. Hooks and commands both call it. There is no second source of truth about what phase we're in or which skills apply.
3. **Gentle enforcement first.** Warnings to stderr are the default. Hard blocks are reserved for two known footguns. Strict mode can be added later if needed; reversing strict-by-default would be painful.
4. **Three commands cover everything.** Quick, strategy, feature. If something doesn't fit, it's either a quick task in disguise or a feature in disguise. There is no fourth.
5. **State is one line.** `.spec/.phase` is `<workflow>:<phase>[:<feature>]`. No JSON, no multiple files, no hidden state — what you see is what's true.
6. **Specs are the backbone.** The `/spec` skill is bundled and always required. Every command reads or writes specs at known points.
7. **Portable, not packaged.** No CLI tool, no npm package, no build step. Pure markdown plus bash plus Claude Code primitives. `install.sh` symlinks files into `~/.claude/` and the project.

---

## Target User

Me. One person. shards-code is personal config made portable, not a framework distributed to others. Decisions favor my workflow, not generality. If someone else adopts it, they should expect to fork.

---

## What We Build

| Feature | Priority | Details in |
|---------|----------|------------|
| **The three commands** (`/code:quick`, `/code:strategy`, `/code:feature`) | P0 | [product-commands.md](product-commands.md) |
| **Keystone routing script** (`bin/detect-context.sh`) | P0 | [tech-detect-context.md](tech-detect-context.md) |
| **Four hooks** (SessionStart, UserPromptSubmit, PreToolUse, Stop) | P0 | [tech-hooks.md](tech-hooks.md) |
| **State management** (`.spec/.phase` + `bin/set-phase.sh`) | P0 | [tech.md](tech.md) |
| **Feature merge tooling** (`bin/merge-feature.sh`) | P0 | [tech.md](tech.md) |
| **Bundled `/spec` skill** | P0 | (already present at `.agents/skills/spec/`) |
| **CLAUDE.md policy** | P0 | (rendered at install time) |
| **`install.sh`** | P0 | [tech.md](tech.md) |
| **`/code:amend`** for mid-flight corrections | P1 | [product-commands.md](product-commands.md) |
| **`.shards/config.json` provider overrides** | P1 | [tech.md](tech.md) |

---

## Implementation Phases

| Phase | Goal | Exit Criteria |
|-------|------|---------------|
| **M1: Foundation** | `detect-context.sh`, `set-phase.sh`, `/code:quick`, SessionStart hook in place | `/code:quick` works end-to-end on a trivial task; routing JSON is correct for the three workflows |
| **M2: Enforcement** | PreToolUse hook (warnings + 2 hard blocks), `/code:strategy` | Strategy run produces global specs; PreToolUse blocks lessons.md writes outside COMPOUND |
| **M3: Lifecycle** | `/code:feature`, `merge-feature.sh`, UserPromptSubmit + Stop hooks, `install.sh` | One full feature cycle (DESIGN → IMPL → COMPOUND) produces correct global spec deltas |
| **v1.1** | `/code:amend`, `.shards/config.json` | Deferred until v1 has proven itself in real use |

---

## The Three Commands

| Command | When to use | What it does | Writes to |
|---|---|---|---|
| **`/code:quick <task>`** | Ad-hoc fixes, ≤5 files, no architecture change | Single-shot implementation. If task is non-trivial (any logic change), writes ephemeral `.spec/.quick-plan.md` first. Reads global specs for context. Light review, no compound, no ship ceremony. | source + `.spec/.quick-plan.md` (gitignored) |
| **`/code:strategy`** | Project bootstrap or major refocus | Runs the DESIGN cluster: research → discuss → spec → optional plan. Re-runnable for refocus. | global specs (`product.md`, `tech.md`, `product-design-language.md`, optional `plan.md`) |
| **`/code:feature <name>`** | Building a real feature | Full lifecycle: feature DESIGN sub-cluster (research, discuss, spec, plan) → human gate → IMPL sub-cluster (verify, work, review, ship, compound). Mostly autonomous between gates. | feature specs → source code → global spec deltas (only on COMPOUND) |

Detailed flows live in [product-commands.md](product-commands.md).

---

## External Dependencies

shards-code orchestrates three external systems. It does not reimplement them.

| System | Surface | Used in |
|---|---|---|
| **Compound Engineering** | `/ce-strategy`, `/ce-ideate`, `/ce-plan`, `/ce-deepen-plan`, `/ce-code-review`, `/ce-commit-push-pr`, `/ce-compound`, `/ce-work` | strategy DESIGN, feature DESIGN:RESEARCH/PLAN, IMPL:REVIEW/SHIP/COMPOUND |
| **Superpowers** | `superpowers:brainstorming`, `superpowers:test-driven-development`, `superpowers:subagent-driven-development`, `superpowers:systematic-debugging` | DISCUSS phases, IMPL:WORK, debugging fallback |
| **Bundled `/spec` skill** | `/spec`, `setup.sh`, `validate.sh` | every SPEC phase, validation gates |

If a skill is missing at runtime, `detect-context.sh` drops it from `skills_to_load` and emits a stderr warning. Workflows degrade, they don't fail.

---

## Non-Goals

- **Not a framework for others.** Personal config made portable. If someone else adopts it, they should fork.
- **Not a CLI tool.** No binaries, no npm. Markdown + bash + Claude Code primitives.
- **Not strict-by-default.** Hard blocks are limited to two known footguns. Reverse if and only if real misuse appears.
- **Not a replacement for any plugin.** CE, Superpowers, `/spec` continue to work standalone.
- **Not project-specific.** Lives in `~/.claude/` plus a per-project `.spec/`.
- **Not a marketplace.** No plugin discovery, no user-installable extensions. The three external dependencies are vetted and pinned at the source-name level.

---

## Product Decisions

1. **Three commands, not five.** `/code:amend` is v1.1; it's only worth its weight if mid-flight correction is a frequent pain. Other "small" commands (`/code:status`, `/code:ship`) belong inside the three commands as internal phases, not as user surface.
2. **Quick mode is stateless.** `/code:quick` does not read or write `.spec/.phase`. It runs against a clean phase or interrupts an active one without changing it. This keeps the no-ceremony promise.
3. **Quick mode plans for any logic change.** Even a small fix that touches behavior writes `.spec/.quick-plan.md` first. Two-line rename / typo / import-only changes can skip the plan. (OPEN — see plan.md.)
4. **Hard blocks are minimal.** Only `lessons.md` outside COMPOUND, global specs outside COMPOUND, and direct `.phase` edits. Everything else is advisory. Adding more blocks requires evidence of repeated misuse.
5. **State file format is `<workflow>:<phase>[:<feature>]`.** Three colon-separated segments, third optional. Empty file = neutral. Written only by `bin/set-phase.sh`.
6. **Feature specs are ephemeral.** Created during DESIGN, consumed during IMPL, merged into global specs during COMPOUND, archived to `.spec/archive/<name>/` after merge. Global specs only absorb cross-cutting decisions, never feature-level detail.
7. **Skill suggestions are stderr-only in v1.** Hooks print "consider loading skill X" to stderr; Claude reads CLAUDE.md plus the suggestion and decides. We don't inject skill content into context automatically. (OPEN — revisit in v1.1.)
8. **`/code:strategy` is re-runnable.** Refocus is a real workflow; strategy must support partial overwrite of global specs without losing branch docs the user wrote by hand.
9. **`/code:feature` requires global specs.** A feature without a project context produces specs that drift. The command errors and points at `/code:strategy` if `product.md` and `tech.md` are missing.
10. **No `.framework.json`.** Routing is hard-coded in `bin/detect-context.sh`. Provider overrides arrive in v1.1 via `.shards/config.json` if and only if a real need surfaces.

---

## Open Questions

These are flagged in `plan.md` and need resolution before or during the relevant milestone:

1. **Quick mode plan threshold.** Currently "any logic change → plan, ≤2 lines literal text → direct." Stricter (always plan) or looser (only architecture-level changes plan)?
2. **Hard-block list.** v1 has 2. Are there other footguns that warrant blocking once we run real workflows?
3. **`.shards/config.json`.** v1 or v1.1?
4. **`/code:amend`.** v1 or v1.1?
5. **Skill auto-loading.** Stderr suggestions only, or actual skill activation when hooks detect a missing skill in the load list?

Defaults if forced: stricter quick threshold, keep blocks minimal, defer config to v1.1, defer amend to v1.1, stderr-only first.

---

## Branch Documents

| Document | Covers |
|----------|--------|
| **[product-commands.md](product-commands.md)** | Detailed UX and behavior for each of the three commands: triggers, sub-phases, gates, output paths, escape hatches. |
