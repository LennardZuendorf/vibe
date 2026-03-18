# Research: Design Agreements

> What architectural decisions have been locked? What's the agreed shape of the framework?

Updated: 2026-03-18

These are decisions made during design discussions. They are the source of truth for product and tech specs. Agreements are numbered for reference.

---

## Core Agreements

1. **Framework of Frameworks** — This is a meta-framework that orchestrates other skills/plugins, not a monolithic system.

2. **Config-driven routing** — A `.spec/.framework.json` file determines which plugin handles each development phase.

3. **Interactive installer** — A `/setup-framework` skill detects installed plugins and lets the user choose providers per phase.

4. **Spec system is the backbone** — `/spec` is always required, always active, never pluggable. Everything reads from and writes to `.spec/`.

5. **Built-in defaults for everything** — The framework works with zero external plugins. Plugins enhance but never gate functionality.

6. **Two clusters, not a flat sequence** — Design Cluster (bootstrap, runs once) front-loads all thinking. Implementation Cluster (per-feature, repeats) executes. *(Updated from original "immutable phase sequence" — phases are now grouped into clusters.)*

7. **Eight phases across two clusters** — Design Cluster: RESEARCH → DISCUSS → SPEC → PLAN. Implementation Cluster: VERIFY → IMPLEMENT → REVIEW → LEARN. *(Updated: added VERIFY and LEARN, grouped into clusters.)*

8. **User approval at cluster boundaries** — User confirms after Design Cluster completes and before each feature's implementation starts. Within the Implementation Cluster, phases flow autonomously. *(Updated from "approval at every gate.")*

9. **Cherry-pick from GSD** — Wave grouping, plan immutability, gap closure, fresh subagents are built-in defaults. Not the XML format, not the 50-file infrastructure.

10. **Cherry-pick from Superpowers** — Phase enforcement and pressure resistance are built-in. Brainstorming, TDD, dual-review are plugin features.

11. **Cherry-pick from Compound Engineering** — LEARN as a distinct phase (not embedded in REVIEW). 10% time budget for knowledge capture. *(New agreement.)*

12. **No new runtime dependencies** — Pure Claude Code skills, bash scripts, markdown, and JSON. No npm, no custom CLI tools.

13. **Global specs are permanent, feature specs are ephemeral** — Global specs (product.md, tech.md) evolve over the project lifetime. Feature specs live in `.spec/features/<name>/` during development and merge into global after shipping. *(New agreement.)*

14. **Verify, don't rewrite** — Before implementation, agents scan the codebase to confirm the feature spec is still valid. They don't redo spec writing. *(New agreement.)*

15. **Support greenfield and rework projects** — Additional optional file types (context.md, docs/, reference/, design scope) support rework projects without requiring a separate mode. *(New agreement from deployment findings.)*

---

## Supported Plugins

| Plugin | Phases It Can Provide | Status |
|--------|----------------------|--------|
| superpowers | DISCUSS (brainstorm), IMPLEMENT (TDD), REVIEW (dual-stage) | Optional |
| feature-dev | RESEARCH (code-explorer), PLAN (code-architect), REVIEW (code-reviewer) | Optional |
| simplify | REVIEW (multi-agent) | Bundled (default review) |
| compound-engineering | LEARN (wiki-based compound step) | Optional, future |

---

## Phase Provider Routing

### Design Cluster (bootstrap)

| Phase | If feature-dev | If superpowers | Default (built-in) |
|-------|---------------|---------------|-------------------|
| RESEARCH | code-explorer agents | — | Parallel Explore agents |
| DISCUSS | — | brainstorm skill | AskUserQuestion |
| SPEC | /spec (always) | /spec (always) | /spec (always) |
| PLAN | competing architects | — | Wave-grouped plan writing |

### Implementation Cluster (per-feature)

| Phase | If feature-dev | If superpowers | If compound-eng | Default (built-in) |
|-------|---------------|---------------|----------------|-------------------|
| VERIFY | code-explorer scan | — | — | Built-in codebase scan |
| IMPLEMENT | — | TDD enforcement | — | Wave-based subagents |
| REVIEW | code-reviewer + confidence | dual-stage review | — | /simplify |
| LEARN | — | — | /ce:compound | Built-in lessons + merge |

---

## Open Questions

Tracked in [questions.md](../questions.md) — 15 open questions grouped by which build wave they block.

Key unresolved:
- VERIFY scope: file existence only, or file+interface? (Q1)
- LEARN merge strategy: auto or user-confirmed? (Q2)
- Feature spec isolation: feature-only or feature+global? (Q3)
- Plugin detection: hardcoded paths or manifest? (Q8)
