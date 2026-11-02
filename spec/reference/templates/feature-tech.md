---
type: feature-tech
feature: {name}
sibling: product.md
parent: ../../tech.md
updated: {YYYY-MM-DD}
---

<!--
  TEMPLATE GUARDRAIL — feature tech.md
  Job: HOW this feature is built (files, contracts, algorithms). Link to product.md for WHAT.
  Do NOT: UX opinions, product requirements, plan units, root architecture essays.
  Omit optional sections entirely when empty — placeholder prose is worse than absence.
-->

# Feature: {Name} — Architecture

{One-paragraph summary: how this feature is built, what files it touches, what the contract is.}

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
<!-- Add Design and Plan header links when design.md / plan.md exist (both optional). -->

---

## Files

```
{path/to/file.ext}        # {what it does}     ~{N} LOC
{path/to/other.ext}       # {what it does}     ~{N} LOC
```

---

## Contract / API

{Interfaces, types, function signatures, data structures.}

```typescript
// {file path}
interface {Name} {
  {field}: {type};
}
```

---

## Implementation Detail

{How it works. Algorithms. Data flow. Code examples with real paths.}

<!--
  Merge markers. To promote a cross-cutting section into the root tech.md, wrap it in the HTML
  comment pair `merge` ... `/merge` (opening and closing marker each on its own line, written out
  in full as in reference/tech.md § merge markers). `vibe-spec promote {name}` appends the wrapped
  blocks. The markers are not written here, because a live block would be promoted as it stands.
-->
-->

<!-- include-when-material: Performance Budget — omit when not applicable -->

## Performance Budget

{Concrete latency / memory / size targets.}

<!-- /include-when-material -->

<!-- include-when-material: Open Questions — omit when no technical blockers -->

## Open Questions

1. **{Question}** — {Context and trade-offs.}

<!-- /include-when-material -->
