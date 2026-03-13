---
type: entrypoint
scope: product
children:
  - product-design.md
updated: 2026-03-13
---

# Engineering Agent — Product

A meta-prompting framework that orchestrates existing AI coding skills, frameworks, and plugins into a single spec-driven development lifecycle. Instead of being yet another monolithic framework, it's **the framework that connects frameworks** — a thin orchestration layer that mix-and-matches the best tools through a centralized config.

**One-liner:** A framework of frameworks — pluggable phases, one workflow, specs as the backbone.

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
1. It enforces a **mandatory phase sequence** (research → spec → plan → implement → review) so you never skip thinking.
2. Each phase is handled by a **configurable provider** — a built-in default, or an installed plugin.
3. The **spec system** (`.spec/`) is the non-negotiable backbone — all phases read from and write to specs.
4. A **centralized installer** detects what you have and lets you choose what to use.

The result: you get GSD's execution patterns, Superpowers' creative tools, Feature-Dev's smart agents, and Simplify's review — without adopting any of them wholesale.

---

## Design Principles

1. **Orchestrate, don't reimplement.** Existing plugins are maintained by their authors. We route to them, not rebuild them. When they ship updates, we get them for free.
2. **Specs are the backbone.** The `/spec` system is always required, never pluggable. Every phase reads from and writes to `.spec/`. This is the one constant.
3. **Config-driven routing.** A single `.spec/.framework.json` determines which plugin handles each phase. No code changes needed to swap providers.
4. **Progressive complexity.** Works out of the box with zero plugins. Each installed plugin unlocks richer behavior. Never forces you to install everything.
5. **Phase gates are sacred.** Research before spec, spec before plan, plan before code. No plugin can bypass this. Hooks enforce it.
6. **Cherry-pick the best patterns.** We adopt proven patterns (GSD's waves, Superpowers' pressure resistance) as built-in defaults. The insights compound.

## Target User

AI-assisted developers who:
- Use Claude Code as their primary development environment
- Have tried multiple prompting frameworks and found each useful but incomplete
- Want disciplined, spec-driven development without monolithic framework overhead
- Value the ability to swap components as better tools emerge
- Want a single `/develop` command that does the right thing regardless of which plugins are installed

## User Stories

1. **As a developer starting a new feature,** I run `/develop <description>` and the framework guides me through research → spec → plan → implement → review, using whichever plugins I've configured.
2. **As a developer setting up a new project,** I run `/setup-framework` and interactively choose which plugins to use for each phase, generating a config file.
3. **As a developer with no plugins installed,** I run `/develop` and get a complete lifecycle using built-in defaults — parallel Explore agents for research, structured questions for discussion, spec-driven planning, wave-based implementation, and self-review.
4. **As a developer who installs a new plugin,** I re-run `/setup-framework`, enable it for specific phases, and my next `/develop` automatically routes to it.
5. **As a developer resuming work,** I run `/develop` and it reads `.spec/.phase` to know where I left off, loads the relevant specs, and continues from that point.

## What We Build

| Feature | Priority | Description |
|---------|----------|-------------|
| **Phase Orchestrator** | P0 | Core `/develop` skill that routes phases to providers |
| **Built-in Defaults** | P0 | Complete lifecycle with zero plugins |
| **Plugin Config System** | P0 | `.framework.json` schema, reader, validator |
| **Interactive Installer** | P1 | `/setup-framework` — detection + config generation |
| **Plugin Adapters** | P1 | Routing adapters for superpowers, feature-dev, simplify |

## Non-Goals

- **Not a plugin marketplace.** We support specific, vetted plugins — not arbitrary extensibility.
- **Not a replacement for any plugin.** Superpowers, feature-dev, simplify continue to work standalone.
- **Not a CLI tool.** No custom binaries, no npm packages. Pure Claude Code skills and shell scripts.
- **Not project-specific.** The framework lives in `.agents/skills/` and works across any project. Only `.spec/.framework.json` is project-specific.
- **Not a new spec format.** We use the existing `/spec` system exactly as-is.

## Product Decisions

1. **Config lives in `.spec/.framework.json`.** Project-specific (different projects may use different plugins), lives alongside the specs it configures.
2. **Built-in defaults for every phase.** Framework is fully functional with zero external plugins. Plugins enhance, never gate.
3. **Phase names are fixed.** RESEARCH, DISCUSS, SPEC, PLAN, IMPLEMENT, REVIEW — these are the constants. What changes is which provider handles each one.
4. **User approval at every gate.** No plugin can auto-advance phases. The human always confirms.
5. **GSD and Superpowers patterns are built-in, not plugin-dependent.** Waves, gap closure, pressure resistance, phase enforcement — these are too useful to be optional.

## Open Questions

- Should we support per-phase model overrides in the config? (e.g., haiku for research, opus for implementation)
- How do we handle plugin version compatibility? (e.g., superpowers v5 vs v6)

## Branch Documents

| Document | Covers |
|----------|--------|
| **[product-design.md](product-design.md)** | Functional design, workflow diagrams, user flows |
