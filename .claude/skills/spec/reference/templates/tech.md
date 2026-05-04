---
type: entrypoint
scope: technical
children:
  - tech-{topic}.md
updated: {YYYY-MM-DD}
---

# {Project Name} — Technical Architecture

## Design Philosophy

1. **{Principle}.** {Technical guiding rule.}
2. **{Principle}.** {Technical guiding rule.}

## Architecture Overview

```
{project}/
├── src/
│   ├── {directory}/          # {annotation: NEW / inherited / extended}
│   └── {directory}/          # {annotation}
├── {other dirs}/
└── .spec/                    # Design docs
```

## Tech Stack

**Inherited:** {Technologies and tools that come from upstream or existing infra.}

**Added:** {New dependencies introduced for this project.}

## What We Build vs Inherit

| Source | Approx. Lines | What |
|--------|---------------|------|
| **{Upstream}** (inherited) | ~{N}k | {What it provides} |
| **{New code}** (this project) | ~{N} | {What we actually write} |

## Key Patterns

- **{Pattern}:** {Brief description.} -> [tech-{topic}.md](tech-{topic}.md)
- **{Pattern}:** {Brief description.} -> [tech-{topic}.md](tech-{topic}.md)

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| {Risk description} | {How to handle it} |

## Branch Documents

| Document | Covers |
|----------|--------|
| **[tech-{topic}.md](tech-{topic}.md)** | {Summary of what this branch covers} |
