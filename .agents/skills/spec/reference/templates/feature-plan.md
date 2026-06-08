---
type: feature-plan
feature: {name}
sibling: tech.md
parent: ../../plan.md
updated: {YYYY-MM-DD}
---

<!--
  TEMPLATE GUARDRAIL — feature plan.md
  Job: HOW to build this feature (units, files, verification). Link up to product.md for WHAT.
  Do NOT: re-litigate product requirements, UX prose, architecture essays, root milestones.
  Omit optional sections entirely when empty — placeholder prose is worse than absence.
-->

# Feature: {Name} — Implementation Plan

{One-paragraph summary: what this feature delivers, in what order, and how units map to root milestones.}

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
<!-- Add Design header link when design.md exists (optional). -->

---

## Problem Frame

{The technical problem this plan solves. Why these units, in this order. One tight paragraph — do not restate product requirements.}

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [{Short name}](product.md#{anchor}) | U1, U2 |
| R2 | [{Short name}](product.md#{anchor}) | U3 |

Every unit below MUST cite the R-IDs it satisfies. Add rows as requirements grow; do not renumber R-IDs.

---

## Key Technical Decisions

1. **{Decision}.** {Trade-off and rationale. Link to tech.md for detail.}
2. **{Decision}.** {…}

---

## Unit Prefix

| Prefix | Feature | Example |
|---|---|---|
| {XX} | {name} | {XX}0, {XX}1, … |

Derive the prefix from the feature slug (2–4 uppercase letters). Register it in the root [plan.md](../../plan.md) unit-prefix table. Never renumber units when reordering work — add new IDs instead.

---

### U1. {Unit name}

**Goal:** {One sentence — what ships in this slice.}

**Requirements:** R1, R2

**Dependencies:** {Prior units or cross-feature units; use `—` when none.}

**Files:**

```
{path/to/file.ext}        # {what changes}
{path/to/other.ext}       # {what changes}
```

**Test scenarios:**

- {Observable behaviour to prove — maps to product requirement scenarios}
- {…}

**Verification:** {Command, test path, script output, or behaviour check that proves this unit done.}

---

### U2. {Unit name}

**Goal:** {…}

**Requirements:** R2

**Dependencies:** U1

**Files:**

```
{path/to/file.ext}
```

**Test scenarios:**

- {…}

**Verification:** {…}

---

<!-- include-when-material: Dependencies — omit when all deps are per-unit above -->

## Dependencies

| Unit | Blocks | Blocked by |
|---|---|---|
| U2 | {downstream units or features} | U1, {other-feature unit} |

Cross-feature dependencies belong here **and** in the root plan critical path.

<!-- /include-when-material -->

<!-- include-when-material: Spec vs Implementation — omit when spec and repo are aligned -->

## Spec vs Implementation

| Gap | Tracked in | Notes |
|---|---|---|
| {Known delta between spec and repo} | U{n} | {One line} |

<!-- /include-when-material -->

<!-- include-when-material: Progress — omit until work begins -->

## Progress

| Unit | Status |
|---|---|
| U1 | NOT STARTED |
| U2 | NOT STARTED |

<!-- /include-when-material -->

<!-- include-when-material: Open Questions — omit when no planning blockers -->

## Open Questions

1. **{Question}** — {Context. Recommendation if any.}

<!-- /include-when-material -->
