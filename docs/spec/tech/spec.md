---
type: hub
applies:
  - "crates/vibe-spec/**"
  - "spec/**"
  - ".vibe/spec.json"
  - "docs/spec/**"
updated: 2026-10-19
---

# Spec Framework — Tech

[tech.md](../tech.md)

Branch doc for the spec tool: root discovery and migration, OpenSpec
interchange, the strictness ratchet, adherence checks, and the four
subagents. **Shipped today:** bash scripts (`validate.sh`, `list-specs.sh`,
`lessons-for.sh`, `scan-merges.sh`, `promote.sh`, `check-drift.sh`) run
cwd-relative or install-relative; see the note at the end.

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
