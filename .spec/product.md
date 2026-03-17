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

The lifecycle is organized into two clusters that run at different cadences:

**Design Cluster (project bootstrap)** — Runs once at project start (or when a new unplanned feature emerges). Front-loads ALL thinking: research the codebase, discuss scope, write global specs AND feature specs for every known feature, create a global implementation plan. The user is heavily involved. This is the heavy lift.

**Implementation Cluster (per-feature, repeating)** — Runs once per feature from the plan. Agents verify the feature spec against the current codebase (a quick check, not a rewrite), then implement, review, and learn. Lightweight and mostly autonomous.

```
┌──────────────── DESIGN CLUSTER (once) ────────────────┐
│                                                        │
│  RESEARCH → DISCUSS → SPEC → PLAN                     │
│                                                        │
│  Outputs:                                              │
│    • Global specs (product.md, tech.md)                │
│    • Feature specs for ALL known features              │
│    • Global plan (sequenced across features)           │
│  User: heavily involved                                │
└────────────────────────┬───────────────────────────────┘
                         │
                         ▼
┌──── IMPLEMENTATION CLUSTER (per feature, repeating) ───┐
│                                                        │
│  VERIFY → IMPLEMENT → REVIEW → LEARN                  │
│                                                        │
│  Inputs: feature spec (read-only, already written)     │
│  VERIFY: quick codebase scan to confirm spec validity  │
│  LEARN: merge feature decisions → global specs         │
│                                                        │
│  Repeat for each feature in the plan                   │
└────────────────────────────────────────────────────────┘
```

**Exception path:** When a new feature emerges that wasn't part of the original plan, a mini Design Cluster runs for just that feature — research, discuss, write its feature spec, amend the global plan — then it joins the normal implementation queue.

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

1. **As a developer starting a new project,** I run `/develop` and the framework bootstraps: researches the codebase, discusses scope with me, writes global specs and feature specs for all known features, and produces a global implementation plan. This is the one heavy session.
2. **As a developer ready to build the next feature,** I run `/develop` and the framework picks the next feature from the plan, verifies its spec against the current codebase, and implements it. No re-speccing, no re-discussing — just verify and go.
3. **As a developer with no plugins installed,** I run `/develop` and get a complete lifecycle using built-in defaults — parallel Explore agents for research, structured questions for discussion, spec-driven planning, wave-based implementation, and multi-agent review.
4. **As a developer who discovers a new feature mid-project,** a mini Design Cluster runs for just that feature — research, discuss, spec, amend plan — then it joins the implementation queue like any other feature.
5. **As a developer resuming work,** I run `/develop` and it reads `.spec/.phase` to know where I left off, loads the relevant feature specs, and continues from that point.
6. **As a developer starting implementation of a feature,** the agent does a quick codebase scan to verify the feature spec still holds — it doesn't redo the spec writing, just confirms nothing has invalidated the plan.
7. **As a developer who shipped a feature,** the LEARN phase extracts lessons, merges feature-level decisions into global specs, archives the feature spec, and the framework moves to the next feature in the plan.
8. **As a developer setting up plugin preferences,** I run `/setup-framework` and interactively choose which plugins to use for each phase, generating a config file.

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

1. **Design Cluster is a project bootstrap.** It runs once at project start, producing global specs, all feature specs, and a global plan. It does NOT run per-feature.
2. **Implementation Cluster is per-feature.** It repeats for each feature in the plan: verify → implement → review → learn. Lightweight and mostly autonomous.
3. **Feature specs are ephemeral.** They live in `.spec/features/<name>/` during development and merge into global specs after shipping. This keeps global specs clean and focused.
4. **Verify, don't rewrite.** Before implementation, agents scan the codebase to confirm the feature spec is still valid. If something has changed, they flag it — they don't silently rewrite the spec.
5. **Unplanned features get a mini bootstrap.** When a new feature emerges mid-project, a scoped Design Cluster runs for just that feature, then it joins the implementation queue.
6. **Config lives in `.spec/.framework.json`.** Project-specific (different projects may use different plugins), lives alongside the specs it configures.
7. **Built-in defaults for every phase.** Framework is fully functional with zero external plugins. Plugins enhance, never gate.
8. **User approval at cluster boundaries.** The user confirms after Design Cluster completes and before each feature's implementation starts. Within the Implementation Cluster, phases flow autonomously.
9. **LEARN is a distinct phase.** Inspired by Compound Engineering's "compound step." Learning is not a checkbox in review — it's a dedicated step that extracts lessons, prunes stale ones, and merges feature decisions into global specs.

## Open Questions

- Should we support per-phase model overrides in the config? (e.g., haiku for research, opus for implementation)
- How do we handle plugin version compatibility? (e.g., superpowers v5 vs v6)
- What is the merge strategy when feature specs conflict with global specs that changed during implementation?

## Branch Documents

| Document | Covers |
|----------|--------|
| **[product-design.md](product-design.md)** | Functional design, workflow diagrams, user flows |
