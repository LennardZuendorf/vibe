---
type: concept
updated: 2026-10-19
---

# Spec Framework Contract

Default root `docs/spec/`; `.spec/` legacy. Layered layout (D44): <!-- literal -->

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
