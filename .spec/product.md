---
type: entrypoint
scope: product
children:
  - product-design.md
updated: 2026-03-14
---

# Engineering Agent — Product

A meta-prompting framework that orchestrates existing AI coding skills, frameworks, and plugins into a single spec-driven development lifecycle. Instead of being yet another monolithic framework, it's **the framework that connects frameworks** — a thin orchestration layer that mix-and-matches the best tools through a centralized config.

**One-liner:** A framework of frameworks — pluggable phases, two clusters, specs as the backbone.

---

## The Problem

Every AI coding framework solves part of the puzzle:
- **GSD** has great execution patterns (waves, gap closure) but ships 50+ files and 12 custom agents — too much overhead.
- **Superpowers** has excellent brainstorming, TDD, and review skills — but they're tightly coupled and hard to use individually.
- **Feature-Dev** has smart specialized agents (explorer, architect, reviewer) — but doesn't enforce a full lifecycle.
- **Simplify** does focused multi-agent review perfectly — but only review.

No single framework gets everything right. Developers end up picking one and missing out on the others, or trying to use multiple and hitting conflicts.

## The Solution

Engineering Agent is the **orchestration layer** that sits above these frameworks:
1. Work is organized into **two clusters**: a Design Cluster (all thinking up front) and an Implementation Cluster (execution with spec verification, not spec rewriting).
2. Each phase within a cluster is handled by a **configurable provider** — a built-in default, or an installed plugin.
3. Specs are split into **global specs** (persistent project truth) and **feature specs** (ephemeral, created during design, merged into global after shipping).
4. The **spec system** (`.spec/`) is the non-negotiable backbone — all phases read from and write to specs.
5. A **centralized installer** detects what you have and lets you choose what to use.

The result: you get GSD's execution patterns, Superpowers' creative tools, Feature-Dev's smart agents, and Simplify's review — without adopting any of them wholesale.

---

## Design Principles

1. **Orchestrate, don't reimplement.** Existing plugins are maintained by their authors. We route to them, not rebuild them. When they ship updates, we get them for free.
2. **Specs are the backbone.** The `/spec` system is always required, never pluggable. Every phase reads from and writes to `.spec/`. This is the one constant.
3. **Front-load the thinking.** All research, discussion, spec writing, and planning happens in the Design Cluster before any code is written. Implementation agents only verify specs, they don't rewrite them.
4. **Global specs are permanent, feature specs are ephemeral.** Global specs (`product.md`, `tech.md`) evolve over the project lifetime. Feature specs live in `.spec/features/<name>/` during development and merge into global after shipping.
5. **Config-driven routing.** A single `.spec/.framework.json` determines which plugin handles each phase. No code changes needed to swap providers.
6. **Progressive complexity.** Works out of the box with zero plugins. Each installed plugin unlocks richer behavior. Never forces you to install everything.
7. **Cluster gates are sacred.** Design must complete before Implementation starts. Within each cluster, phases run in order. No skipping.
8. **Cherry-pick the best patterns.** We adopt proven patterns (GSD's waves, Superpowers' pressure resistance, Compound Engineering's learning phase) as built-in defaults.

## Two Clusters

The lifecycle is organized into two clusters, not a flat sequence of phases:

**Design Cluster** — Front-loaded thinking. All research, discussion, spec writing, and planning happens here. This cluster produces feature specs and an implementation plan. The user is heavily involved.

**Implementation Cluster** — Execution. Agents verify the feature spec against the current codebase (not rewrite it), then implement, review, and learn. The user approves at cluster boundaries, not at every micro-step.

```
┌─────────────── DESIGN CLUSTER ───────────────┐   ┌────────── IMPLEMENTATION CLUSTER ──────────┐
│                                               │   │                                            │
│  RESEARCH → DISCUSS → SPEC → PLAN            │──▶│  VERIFY → IMPLEMENT → REVIEW → LEARN       │
│                                               │   │                                            │
│  Outputs: feature specs, implementation plan  │   │  Inputs: feature specs (read-only)          │
│  User: heavily involved                       │   │  Pre-step: codebase scan to confirm spec    │
└───────────────────────────────────────────────┘   └────────────────────────────────────────────┘
```

## Global vs Feature Specs

| Aspect | Global Specs | Feature Specs |
|--------|-------------|---------------|
| **Location** | `.spec/product.md`, `.spec/tech.md`, `.spec/product-*.md`, `.spec/tech-*.md` | `.spec/features/<name>/product.md`, `.spec/features/<name>/tech.md`, `.spec/features/<name>/plan.md` |
| **Lifetime** | Persistent — evolves over the project lifetime | Ephemeral — created during Design, merged after shipping |
| **Scope** | Entire project: architecture, design system, conventions, cross-cutting concerns | Single feature: what it does, how it's built, implementation plan |
| **Who writes** | Design Cluster (initial), LEARN phase (updates after merge) | Design Cluster only |
| **Who reads** | Everyone, always | Implementation Cluster for that feature |
| **Merging** | N/A | After REVIEW+LEARN, feature decisions that affect global architecture get merged into global specs. Feature spec directory is archived. |

**Why this separation matters:**
- Implementation agents don't need to re-read the entire project spec. They get a focused feature spec.
- Global specs don't bloat with feature-level detail during development. They only absorb the cross-cutting decisions after a feature ships.
- Multiple features can be designed in parallel without spec conflicts — each has its own directory.
- Feature specs can be discarded if a feature is abandoned, without polluting global specs.

## Target User

AI-assisted developers who:
- Use Claude Code as their primary development environment
- Have tried multiple prompting frameworks and found each useful but incomplete
- Want disciplined, spec-driven development without monolithic framework overhead
- Value the ability to swap components as better tools emerge
- Want a single `/develop` command that does the right thing regardless of which plugins are installed

## User Stories

1. **As a developer starting a new feature,** I run `/develop <description>` and the framework guides me through the Design Cluster (research → discuss → spec → plan), then the Implementation Cluster (verify → implement → review → learn).
2. **As a developer setting up a new project,** I run `/setup-framework` and interactively choose which plugins to use for each phase, generating a config file.
3. **As a developer with no plugins installed,** I run `/develop` and get a complete lifecycle using built-in defaults — parallel Explore agents for research, structured questions for discussion, spec-driven planning, wave-based implementation, and multi-agent review.
4. **As a developer who installs a new plugin,** I re-run `/setup-framework`, enable it for specific phases, and my next `/develop` automatically routes to it.
5. **As a developer resuming work,** I run `/develop` and it reads `.spec/.phase` to know where I left off, loads the relevant feature specs, and continues from that point.
6. **As a developer starting implementation,** the agent scans the codebase to verify the feature spec still holds — it doesn't redo the spec writing, just confirms nothing has changed that invalidates the plan.
7. **As a developer who shipped a feature,** the LEARN phase extracts lessons and merges feature-level decisions into global specs, then archives the feature spec directory.

## What We Build

| Feature | Priority | Description |
|---------|----------|-------------|
| **Cluster Orchestrator** | P0 | Core `/develop` skill that manages Design and Implementation clusters |
| **Feature Spec System** | P0 | `.spec/features/<name>/` creation, isolation, and merge-back |
| **Built-in Defaults** | P0 | Complete lifecycle with zero plugins |
| **Plugin Config System** | P0 | `.framework.json` schema, reader, validator |
| **Spec Verification** | P0 | Pre-implementation codebase scan that confirms feature spec validity |
| **Interactive Installer** | P1 | `/setup-framework` — detection + config generation |
| **Plugin Adapters** | P1 | Routing adapters for superpowers, feature-dev, simplify |

## Non-Goals

- **Not a plugin marketplace.** We support specific, vetted plugins — not arbitrary extensibility.
- **Not a replacement for any plugin.** Superpowers, feature-dev, simplify continue to work standalone.
- **Not a CLI tool.** No custom binaries, no npm packages. Pure Claude Code skills and shell scripts.
- **Not project-specific.** The framework lives in `.agents/skills/` and works across any project. Only `.spec/.framework.json` is project-specific.
- **Not a new spec format.** We use the existing `/spec` system exactly as-is, extended with feature directories.

## Product Decisions

1. **Two clusters, not a flat sequence.** Design Cluster front-loads all thinking. Implementation Cluster executes. This prevents the pattern of re-doing spec work during implementation.
2. **Feature specs are ephemeral.** They live in `.spec/features/<name>/` during development and merge into global specs after shipping. This keeps global specs clean and focused.
3. **Verify, don't rewrite.** Before implementation, agents scan the codebase to confirm the feature spec is still valid. If something has changed, they flag it — they don't silently rewrite the spec.
4. **Config lives in `.spec/.framework.json`.** Project-specific (different projects may use different plugins), lives alongside the specs it configures.
5. **Built-in defaults for every phase.** Framework is fully functional with zero external plugins. Plugins enhance, never gate.
6. **User approval at cluster boundaries.** The user confirms after Design Cluster completes and after Implementation Cluster completes. Within clusters, phases flow without mandatory gates (but can be configured to pause).
7. **LEARN is a distinct phase.** Inspired by Compound Engineering's "compound step." Learning is not a checkbox in review — it's a dedicated step that extracts lessons, prunes stale ones, and merges feature decisions into global specs.

## Open Questions

- Should we support per-phase model overrides in the config? (e.g., haiku for research, opus for implementation)
- How do we handle plugin version compatibility? (e.g., superpowers v5 vs v6)
- What is the merge strategy when feature specs conflict with global specs that changed during implementation?

## Branch Documents

| Document | Covers |
|----------|--------|
| **[product-design.md](product-design.md)** | Functional design, workflow diagrams, user flows |
