---
type: hub
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-11-12
---

# Spec Framework — Tech

[tech.md](../tech.md)

Branch doc for spec discovery, storage, lifecycle, validation and adapters.
The shared Rust library serves central `vibe spec`, compatibility `vibe-spec`
and local stdio MCP. Projects/storage/MCP are implemented, locally verified and
independently reviewed on this branch. Release distribution and installer delivery
remain separate. Bundled bash scripts remain legacy flat-root fallbacks.
See [Shipped today](spec/spec-shipped.md) for delivered F-row history.

## Notes

- [Root and discovery](spec/spec-root-discovery.md) — discovery order and superpowers artifact root
- [Migrate](spec/spec-migrate.md) — migrate to docs/spec, population assertion, layered layout
- [OpenSpec interchange](spec/spec-openspec.md) — grammar, mapping table, export and import, lossy list
- [Safety of promote, export, import, check-drift](spec/spec-openspec-safety.md) — fail-closed rules and divergences from the oracles
- [Strictness](spec/spec-strictness.md) — levels, registry, allow markers
- [Adherence checks](spec/spec-adherence.md) — layout, budgets, links, names, typed notes, trace
- [Subagents](spec/spec-subagents.md) — spec-promoter and spec-health
- [Hooks](spec/spec-hooks.md) — shim, wiring, validate hooks, router, tooth
- [Setup and commands](spec/spec-setup-commands.md) — setup flags, git hook, OpenCode files, command list
- [Shipped today](spec/spec-shipped.md) — what each delivered row added
- [Projects and MCP](spec/spec-projects-mcp.md) — context, repo-first/global-first storage, reconciliation and local stdio; release distribution remains separate
