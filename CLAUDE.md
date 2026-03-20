# Engineering Agent — Meta-Prompting Framework

A framework of frameworks for AI-assisted software engineering. Orchestrates existing skills and plugins (superpowers, feature-dev, simplify) into a unified, spec-driven development lifecycle.

## Philosophy

- **Orchestrate, don't reimplement.** Route to existing plugins, don't rebuild them.
- **Specs are the backbone.** `.spec/` is the source of truth. No code without specs.
- **Phase gates are sacred.** Research → Discuss → Spec → Plan → Implement → Review. No skipping.

## Skills

| Skill | Purpose | When to Use |
|-------|---------|-------------|
| `/spec` | Navigate and maintain design specs in `.spec/` | Before writing ANY code or making design decisions |
| `/develop` | Full feature development lifecycle with pluggable phases | Starting any feature, bug fix, or refactoring task |
| `/setup-framework` | Interactive plugin detection and config generation | Setting up a new project or changing plugin configuration |
| `/simplify` | Multi-agent code review (reuse, quality, efficiency) | During REVIEW phase or standalone code review |

## Development Workflow

```
/develop <feature-description>
  Phase 1: RESEARCH    → Explore codebase, understand context
  Phase 2: DISCUSS     → Clarify scope, surface ambiguity
  Phase 3: SPEC        → Write product + tech specs via /spec
  Phase 4: PLAN        → Create wave-grouped implementation plan
  Phase 5: IMPLEMENT   → Execute plan wave by wave
  Phase 6: REVIEW      → Verify, test, review, validate specs
```

Each phase is handled by a **configurable provider** (built-in default or installed plugin). Configuration lives in `.spec/.framework.json`.

## Plugin System

The framework supports these plugins as phase providers:

| Plugin | Phases | Status |
|--------|--------|--------|
| **simplify** | REVIEW | Bundled |
| **superpowers** | DISCUSS, IMPLEMENT (TDD), REVIEW | Optional |
| **feature-dev** | RESEARCH, PLAN, REVIEW | Optional |

Run `/setup-framework` to configure which plugins handle which phases.

## Rules

1. **Specs before code** — Never write implementation code without specs in `.spec/`
2. **Read before write** — Always read existing specs and lessons before any work
3. **Phase gates enforced** — Hooks prevent writes during wrong phases
4. **Config-driven routing** — `.framework.json` determines phase providers
5. **Built-in defaults** — Framework works with zero plugins installed
6. **Lessons are mandatory** — Update `.spec/lessons.md` after every correction

## Context Files

- @.agents/skills/spec/SKILL.md — Spec system skill
- @.agents/skills/develop/SKILL.md — Development lifecycle skill (config-aware)
- @.claude/settings.json — Hook and permission configuration
- @.spec/.framework.json — Plugin routing configuration (per-project)
