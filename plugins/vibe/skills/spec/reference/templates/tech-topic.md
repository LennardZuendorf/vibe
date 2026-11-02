---
type: tech-topic
parent: tech.md
scope: {topic}
covers: {comma-separated list}
updated: {YYYY-MM-DD}
---

<!--
  TEMPLATE GUARDRAIL — tech-{topic}.md
  Job: cross-cutting technical contract that every feature integrates with.
  Do NOT: one feature's detail; that belongs in the feature folder.
  Use in a flat layout only; a layered root keeps the hubs that `vibe-spec init` writes.
-->

# {topic} — Tech

One paragraph: what this cross-cutting technical concern governs.

## Contract

{Invariant every feature must satisfy.}

## File Layout

```
path/to/shared/
├── file.ext     ← purpose
```

## Integration Points

| Feature | How it integrates |
|---|---|
| {name} | … |

**When to create:** Only for concerns that every feature must integrate with (auth, observability, error handling).
