---
type: product-topic
parent: product.md
scope: {topic}
covers: {comma-separated list}
updated: {YYYY-MM-DD}
---

<!--
  TEMPLATE GUARDRAIL — product-{topic}.md
  Job: cross-cutting product concern that spans every feature.
  Do NOT: one feature's detail; that belongs in the feature folder.
  Use in a flat layout only; a layered root keeps the hubs that `vibe-spec init` writes.
-->

# {topic} — Product

One paragraph: why this cross-cutting concern exists and who it affects.

## Principles

- Principle 1
- Principle 2

## Requirements

### Requirement: {title}

{The system} SHALL/MUST {observable behaviour}.

#### Scenario: {title}

**Given** …
**When** …
**Then** …

**When to create:** Only when a concern truly spans every feature — not as a substitute for a feature folder.
