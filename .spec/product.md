---
type: entrypoint
scope: product
children: []
updated: 2026-03-13
---

# Engineering Agent — Product

A meta-framework for AI-assisted software engineering that orchestrates existing skills, frameworks, and plugins into a unified development lifecycle. Instead of reimplementing what others build well, it mix-and-matches the best tools through a centralized config.

**One-liner:** A framework of frameworks — pluggable phases, one workflow.

---

## Design Principles

1. **Orchestrate, don't reimplement.** Existing plugins (superpowers, feature-dev, simplify) are maintained by their authors. We route to them, not rebuild them. When they ship updates, we get them for free.
2. **Specs are the backbone.** The `/spec` system is always required, never pluggable. Every phase reads from and writes to `.spec/`. This is the one constant.
3. **Config-driven routing.** A single `.spec/.framework.json` determines which plugin handles each phase. No code changes needed to swap providers.
4. **Progressive complexity.** Works out of the box with built-in defaults (zero plugins). Each installed plugin unlocks richer behavior at that phase. Never forces you to install everything.
5. **Phase gates are sacred.** Research before spec, spec before plan, plan before code. No plugin can bypass this sequence. The hooks enforce it regardless of which providers are active.

## Target User

AI-assisted developers who:
- Use Claude Code as their primary development environment
- Have tried multiple prompting frameworks (GSD, superpowers, feature-dev) and found each useful but incomplete
- Want the discipline of spec-driven development without the overhead of a monolithic framework
- Value the ability to swap components as better tools emerge

## What We Build

| Feature | Priority | Details in |
|---------|----------|------------|
| **Plugin System** | P0 | [product-plugins.md](product-plugins.md) |
| **Setup/Install Utility** | P0 | [product-installer.md](product-installer.md) |
| **Phase Orchestrator** | P0 | _(core of /develop skill)_ |
| **Built-in Defaults** | P0 | _(fallbacks when no plugin installed)_ |

## Implementation Phases

| Phase | Goal | Exit Criteria |
|-------|------|---------------|
| **1: Core Framework** | Plugin config, phase routing, built-in defaults | `/develop` works with zero plugins using built-in providers |
| **2: Plugin Adapters** | Integrate superpowers, feature-dev, simplify | Each plugin can be configured as a phase provider |
| **3: Interactive Installer** | `/setup-framework` skill with detection + config generation | User can interactively choose providers per phase |

## Non-Goals

- **Not a plugin marketplace.** We support specific, vetted plugins — not arbitrary extensibility.
- **Not a replacement for any plugin.** Superpowers, feature-dev, simplify continue to work standalone. We just orchestrate them.
- **Not a CLI tool.** No custom binaries, no npm packages. Pure Claude Code skills and shell scripts.
- **Not project-specific.** The framework lives in `.agents/skills/` and works across any project. Only `.spec/.framework.json` is project-specific.

## Product Decisions

1. **Config lives in `.spec/.framework.json`.** It's project-specific (different projects may use different plugins) and lives alongside the specs it configures.
2. **Built-in defaults for every phase.** The framework must be fully functional with zero external plugins. Plugins enhance, never gate.
3. **Phase names stay the same.** RESEARCH, SPEC, PLAN, IMPLEMENT, REVIEW — these are the constants. What changes is which provider handles each one.
4. **User approval still required at every gate.** No plugin can auto-advance phases. The human always confirms.

## Open Questions

- Should we support per-phase model overrides in the config? (e.g., use haiku for research, opus for implementation)
- How do we handle plugin version compatibility? (e.g., superpowers v5 vs v6)

## Branch Documents

| Document | Covers |
|----------|--------|
| **[product-plugins.md](product-plugins.md)** | Plugin system: discovery, configuration, routing |
| **[product-installer.md](product-installer.md)** | Interactive setup experience and config generation |
