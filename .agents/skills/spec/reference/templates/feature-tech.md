---
type: feature-tech
feature: {name}
sibling: product.md
parent: ../../tech.md
updated: {YYYY-MM-DD}
---

# Feature: {Name} — Architecture

{One-paragraph summary: how this feature is built, what files it touches, what's the contract.}

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)

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

<!-- merge -->
{Sections marked merge get promoted into root tech.md during COMPOUND.
Use this for cross-cutting decisions that affect the whole project.}
<!-- /merge -->

---

## Performance Budget

{If applicable. Concrete latency / memory / size targets.}

---

## Open Questions

1. **{Question}** — {Context and trade-offs.}
