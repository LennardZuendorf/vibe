# Research: Framework Insights by Workflow Phase

Captured: 2026-03-13

This document clusters what we learned from each framework, organized by the workflow phases they influence. Use this as the reference when deciding what to build into core vs. what stays pluggable.

---

## Phase: RESEARCH

> Goal: Understand the problem space, existing code, and constraints before writing anything.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **Feature-Dev** | Spawns specialized `code-explorer` agents that search by file type, pattern, and dependency graph. Returns structured findings. | Plugin | Adapter routes to feature-dev's explorer agents |
| **Superpowers** | Uses `brainstorm` skill to generate divergent requirement interpretations before converging. | Plugin | Adapter routes to superpowers brainstorm |
| **GSD** | Dispatches 3+ parallel research agents with distinct search mandates. Merges findings into structured summary. | Built-in | Parallel Explore agents with mandate-based prompts |
| **Our Framework** | Writes research to `.spec/research/` so it survives session boundaries. | Built-in (core) | Always persist, regardless of provider |

**Built-in behavior:** Spawn parallel Explore agents (Glob + Grep + WebSearch). Write findings to `.spec/research/*.md`. Present to user for confirmation.

**Plugin enhancement:** Feature-dev adds smarter agent types. Superpowers adds requirements brainstorming.

---

## Phase: DISCUSS

> Goal: Clarify ambiguity, align on scope, surface hidden requirements before speccing.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **Superpowers** | `/brainstorm` — divergent thinking, multiple perspectives, then convergence. Structured creative exploration. | Plugin | Adapter calls brainstorm skill |
| **Feature-Dev** | Phase 3 "clarifying questions" — structured Q&A to reduce ambiguity before architecture. | Plugin | Adapter generates structured questions |
| **GSD** | No explicit discuss phase. | — | — |
| **Our Framework** | AskUserQuestion with structured prompts covering scope, constraints, and preferences. | Built-in | Multi-question AskUserQuestion |

**Built-in behavior:** Structured AskUserQuestion covering: scope boundaries, user constraints, priority trade-offs.

**Plugin enhancement:** Superpowers brainstorm adds creative divergent thinking. Feature-dev adds systematic clarification.

---

## Phase: SPEC

> Goal: Write product and tech specs that fully describe the feature. Always uses /spec.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **All** | None of the evaluated frameworks have a spec system. | — | — |
| **Our Framework** | `/spec` — product.md (what/why), tech.md (how), branch docs, validation, cross-refs. | Built-in (core) | Non-negotiable. Always active. |

**Built-in behavior:** `/spec` handles everything. Product specs have zero code. Tech specs reference real paths. Validation enforced.

**Plugin enhancement:** None. This phase is not pluggable.

---

## Phase: PLAN

> Goal: Create concrete implementation plan with milestones, tasks, and validation criteria.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **Feature-Dev** | Spawns 2-3 competing `code-architect` agents with different mandates (minimal, balanced, comprehensive). User picks best proposal. | Plugin | Adapter spawns architect agents, presents proposals |
| **Superpowers** | Writes plan as if for a "dumb executor" — extremely explicit, no ambiguity allowed. | Pattern (built-in) | Adopt the explicitness principle in plan writing |
| **GSD** | Wave-based task grouping. Tasks within a wave have no dependencies (parallelizable). Waves run sequentially. Plan is immutable once approved. | Built-in | Wave grouping + immutability are defaults |
| **Our Framework** | Plans live in `.spec/plan.md` and `plan-{topic}.md`. Progress tracked with checkboxes. | Built-in (core) | Always write to spec files |

**Built-in behavior:** Wave-grouped plan in `.spec/plan.md`. Explicit task descriptions. Plan immutability during implementation (changes require explicit update step).

**Plugin enhancement:** Feature-dev adds competing architecture proposals. Useful for complex features where the approach isn't obvious.

---

## Phase: IMPLEMENT

> Goal: Write code following the plan, milestone by milestone.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **GSD** | Fresh subagent per task (clean context). Wave-based execution (wave N completes before wave N+1). Gap closure (stop if plan doesn't match reality). | Built-in | Core execution model |
| **Superpowers** | TDD enforcement — write tests first, then implementation. Dual-pass: first pass writes, second pass reviews. | Plugin | Adapter enforces test-first when enabled |
| **Feature-Dev** | Standard implementation with checkpoints. | — | No unique pattern to adopt |
| **Our Framework** | Worktree-isolated subagents for independent tasks. Background test runs between waves. | Built-in | Worktree isolation for file-level independence |

**Built-in behavior:** Wave-based execution. Fresh subagent per task. Gap closure (stop and update plan if reality diverges). Background test runs.

**Plugin enhancement:** Superpowers TDD adds test-first discipline. Optional per project.

---

## Phase: REVIEW

> Goal: Validate implementation is correct, clean, and complete.

| Framework | What It Does | Adopt? | How |
|-----------|-------------|--------|-----|
| **Simplify** | 3 parallel review agents (reuse, quality, efficiency). Each returns focused findings. Merged into action items. | Built-in (bundled) | `/simplify` is the default review provider |
| **Superpowers** | Dual-stage: Stage 1 checks spec compliance. Stage 2 checks code quality. Both must pass. | Plugin | Adapter runs two-stage review |
| **Feature-Dev** | `code-reviewer` agent with confidence scoring (0-100). Blocks merge if confidence < threshold. | Plugin | Adapter runs reviewer with threshold |
| **GSD** | Goal-backward verification — "what must be TRUE?" then verify each condition in code. | Built-in | Adopt as review methodology |
| **Our Framework** | Spec compliance check (re-read specs, verify each requirement is implemented). | Built-in (core) | Always runs, regardless of provider |

**Built-in behavior:** Goal-backward verification + spec compliance check + `/simplify` multi-agent review. Self-review checklist.

**Plugin enhancement:** Feature-dev adds confidence scoring. Superpowers adds explicit two-stage gating.

---

## Cross-Cutting Patterns (Not Phase-Specific)

| Pattern | Source | Category | Description |
|---------|--------|----------|-------------|
| File-based communication | Ours | Built-in (core) | All state in `.spec/`. Sessions are disposable, files are permanent. |
| Phase gates via hooks | Superpowers | Built-in (core) | PreToolUse hooks prevent writes in wrong phases. |
| Pressure resistance | Superpowers | Built-in (core) | Politely refuse "just skip to coding." |
| Fresh subagent per task | GSD | Built-in | Each task gets clean context. Prevents rot. |
| Compact summaries | GSD | Built-in | Subagents return ~95% reduced output. |
| Config-driven routing | Ours | Built-in (core) | `.framework.json` determines providers. |
| Lessons tracking | Ours | Built-in (core) | `.spec/lessons.md` compounds knowledge. |
