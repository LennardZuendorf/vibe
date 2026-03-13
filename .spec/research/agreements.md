# Research: Design Agreements

Captured: 2026-03-13

These are decisions made during the initial design discussion. They are the source of truth for product and tech specs.

## Core Agreements

1. **Framework of Frameworks** — This is a meta-framework that orchestrates other skills/plugins, not a monolithic system.

2. **Config-driven routing** — A `.spec/.framework.json` file determines which plugin handles each development phase.

3. **Interactive installer** — A `/setup-framework` skill detects installed plugins and lets the user choose providers per phase.

4. **Spec system is the backbone** — `/spec` is always required, always active, never pluggable. Everything reads from and writes to `.spec/`.

5. **Built-in defaults for everything** — The framework works with zero external plugins. Plugins enhance but never gate functionality.

6. **Phase sequence is immutable** — RESEARCH → SPEC → PLAN → IMPLEMENT → REVIEW. No plugin can reorder or skip phases.

7. **User approval at every gate** — Phase transitions always require human confirmation.

8. **Cherry-pick from GSD** — Wave grouping, plan immutability, gap closure, fresh subagents are built-in defaults. Not the XML format, not the 50-file infrastructure.

9. **Cherry-pick from Superpowers** — Phase enforcement and pressure resistance are built-in. Brainstorming, TDD, dual-review are plugin features.

10. **No new runtime dependencies** — Pure Claude Code skills, bash scripts, markdown, and JSON. No npm, no custom CLI tools.

## Supported Plugins (Initial Set)

| Plugin | Phases It Can Provide |
|--------|----------------------|
| superpowers | DISCUSS (brainstorm), IMPLEMENT (TDD), REVIEW (dual-stage) |
| feature-dev | RESEARCH (code-explorer), PLAN (code-architect), REVIEW (code-reviewer) |
| simplify | REVIEW (multi-agent) — bundled |

## Phase Provider Routing (Agreed)

| Phase | If feature-dev | If superpowers | Default (built-in) |
|-------|---------------|---------------|-------------------|
| RESEARCH | code-explorer agents | brainstorm | Parallel Explore agents |
| DISCUSS | clarifying questions | brainstorm skill | AskUserQuestion |
| SPEC | /spec (always) | /spec (always) | /spec (always) |
| PLAN | competing architects | plan-for-dumb-executor | Direct plan writing |
| IMPLEMENT | standard | TDD enforcement | Wave-based subagents |
| REVIEW | code-reviewer + confidence | dual-stage review | /simplify |

## Open Questions (Unresolved)

- Per-phase model overrides in config?
- Plugin version compatibility handling?
- Exact adapter interface for delegating to plugin skills?
