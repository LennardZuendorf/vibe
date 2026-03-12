# Engineering Agent — Spec-Driven Development Framework

A meta-framework for AI-assisted software engineering that enforces what senior engineers do naturally: research first, plan second, code last.

## Philosophy

This framework treats specs as the source of truth. No code gets written until specs exist, are reviewed, and are approved. Every phase has gates enforced by hooks that prevent skipping ahead.

## Framework Skills

| Skill | Purpose | When to Use |
|-------|---------|-------------|
| `/spec` | Navigate and maintain design specs in `.spec/` | Before writing ANY code or making design decisions |
| `/develop` | Full feature development lifecycle with phase gates | Starting any feature, bug fix, or refactoring task |

## Development Workflow

All feature work follows this mandatory sequence:

```
/develop <feature-description>
  Phase 1: RESEARCH    → Explore codebase, understand context (subagents)
  Phase 2: SPEC        → Write product + tech specs via /spec
  Phase 3: PLAN        → Create implementation plan, get approval
  Phase 4: IMPLEMENT   → Write code following the plan
  Phase 5: REVIEW      → Run /simplify, validate, self-review
```

## Rules

1. **Specs before code** — Never write implementation code without specs in `.spec/`
2. **Read before write** — Always read existing specs and lessons before any work
3. **Phase gates are enforced** — Hooks prevent writes during research/spec/plan phases
4. **Subagents for research** — Use parallel Explore agents for codebase understanding
5. **Validate continuously** — Run `/spec validate` after any spec change
6. **Lessons are mandatory** — Update `.spec/lessons.md` after every correction or mistake

## Context Files

- @.agents/skills/spec/SKILL.md — Spec system skill
- @.agents/skills/develop/SKILL.md — Development lifecycle skill
- @.claude/settings.json — Hook and permission configuration
