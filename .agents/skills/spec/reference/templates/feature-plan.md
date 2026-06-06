---
type: feature-plan
feature: {name}
parent: ../../plan.md
updated: {YYYY-MM-DD}
---

# Feature: {Name} — Plan

{One paragraph: what this feature delivers, in what order, and how units map to milestones in the root plan.}

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

---

## Unit prefix

| Prefix | Feature | Example |
|---|---|---|
| {XX} | {name} | {XX}0, {XX}1, … |

Derive the prefix from the feature slug (2–4 uppercase letters). Register it in the root [plan.md](../../plan.md) unit-prefix table. Never renumber units when reordering work — add new IDs instead.

---

## Units

| ID | Summary | Depends on | Milestone |
|---|---|---|---|
| {XX}0 | {First unit — migration, scaffold, or foundation} | — | M{n} |
| {XX}1 | {Next unit} | {XX}0 | M{n} |
| {XX}2 | {…} | {XX}1 | M{n} |

Each unit is one reviewable slice: testable, committable, citeable in commits and tests during `impl`.

---

## Dependencies

| Unit | Blocks | Blocked by |
|---|---|---|
| {XX}1 | {downstream units or features} | {XX}0, {other-feature unit} |

Cross-feature dependencies belong here **and** in the root plan critical path. Do not duplicate milestone narrative — link up.

---

## Spec vs implementation

| Gap | Tracked in | Notes |
|---|---|---|
| {Known delta between spec and repo} | {unit ID} | {One line} |

Document gaps honestly. Close them via units; do not hide drift in prose.

---

## Verification

| Unit | Evidence |
|---|---|
| {XX}0 | {command, test file, or observable behaviour} |
| {XX}1 | {…} |

---

## Open questions

1. **{Question}** — {Context. Recommendation if any.}
