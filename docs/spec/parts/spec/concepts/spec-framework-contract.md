---
type: concept
updated: 2026-11-12
---

# Spec Framework Contract

Back to [spec](../spec.md).

Current default root `docs/spec/`; `.spec/` is deprecated compatibility only. <!-- literal -->
This repository retains `docs/spec/`. Layered layout (D44):

```text
docs/spec/
├── README.md · AGENTS.md · product.md · tech.md · design.md · plan.md · lessons.md
├── areas/ flows/ concepts/ platform/ tech/ decisions/ questions/   (on demand)
├── parts/<part>/                                                   (optional)
├── features/<feature>/{product,tech,design,plan,research}.md        (fallback, D45)
└── archive/<feature>/                                              (fallback, transient)
docs/superpowers/{specs,plans}/                                     (default feature layer, D45/D46)
```

The marker grammar and every validation rule live once, in `vibe-spec`; the
framework owns no cursor, no injection, no block composition. Full
discovery order, migration, OpenSpec interchange, and validation:
[tech-spec.md](../../../tech/spec.md).

## Project storage and access

- **Storage:** repo-first keeps durable and feature specs in the code repository;
  global-first keeps both outside it, under a registered project and code branch.
- **Checkout:** repo-first uses selected or matching ambient checkout files.
  Otherwise project-only selection follows the main checkout's actual branch.
  Selecting another branch never switches code automatically.
- **Sharing:** clones/worktrees of the same global-first project and branch share
  its complete spec. Different branches evolve independently.
- **Reconciliation:** explicit snapshots bind code/spec evidence. Later reads
  may reconcile a newly observed direct two-parent merge with unique clean
  unchanged evidence; first observation only baselines. Uncertain histories need
  explicit saved-plan application. Conflicts remain readable/resolvable through
  CLI/MCP and never block or reverse code merges. Stored specs survive code-ref
  deletion. Completed reconciliation requires zero validation errors.
- **Interfaces:** central `vibe spec` and local Rust MCP use one application API
  for setup, context, documents, validation, feature lifecycle and reconciliation.
- **Boundary:** no Flow/Instruct implementation, general Spaces/config, remote
  server or harness MCP configuration belongs to this expansion.

Implementation choices and acceptance checks live in
[projects and Rust MCP](../../../tech/spec/spec-projects-mcp.md).
The [feature sequence](spec-feature-sequence.md) records implemented, documented
and locally verified scope. Independent review passed.
Release distribution and installer delivery remain separate.
Release distribution, general Spaces/config, remote
transport and mode migration remain outside scope.
