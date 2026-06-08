---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
children: []
updated: {YYYY-MM-DD}
---

<!--
  TEMPLATE GUARDRAIL — root plan.md
  Job: cross-feature sequencing (WHAT ships WHEN). Link to feature plan.md for units.
  Do NOT: feature unit tables, step-by-step tasks, product re-litigation, architecture detail.
  Omit optional sections entirely when empty — placeholder prose is worse than absence.
-->

# {Project Name} — Implementation Plan

{One paragraph: current delivery focus, how features compose, and what milestone is active.}

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md)

**Feature plans (unit-level detail lives here, not duplicated below):**

| Feature | Product | Plan | Status |
|---|---|---|---|
| [{name}](features/{name}/product.md) | `.spec/` + {layer} | [plan.md](features/{name}/plan.md) | {planned / in progress / done} |

---

<!-- include-when-material: Validation Summary — omit until a meaningful audit exists -->

## Validation Summary

{What is built vs what remains. Honest spec-vs-repo gap if any.}

<!-- /include-when-material -->

## Feature Boundaries

{ASCII diagram or table: what each feature owns vs does not own. Prevents scope bleed.}

```
{feature-a}  ── owns ──>  {paths / concerns}
{feature-b}  ── owns ──>  {paths / concerns}
```

| Layer | Owns | Does not own |
|---|---|---|
| **{feature}** | {paths, scripts, contracts} | {neighbour features, root concerns} |

---

## Milestones

| ID | Goal | Features | Exit |
|---|---|---|---|
| M0 | {Foundation} | {names} | {Observable exit criteria} |
| M1 | {…} | {names} | {…} |

Milestones are delivery phases at the **root** layer. Subdivide parallel tracks as M4a / M4b when needed. Map feature units to milestones in each feature plan — do not duplicate unit tables here.

---

## Unit Prefixes

| Prefix | Feature | Plan |
|---|---|---|
| {XX} | {name} | [features/{name}/plan.md](features/{name}/plan.md) |

Register every feature prefix once. Agents cite unit IDs (`{XX}1`) in commits and tests during `impl`.

---

<!-- include-when-material: Critical Path — omit when no hard cross-feature dependencies yet -->

## Critical Path

```
{dependency chain — e.g. SF0 → VF1 → AI0 → U3}
```

Hard dependencies only. Cross-feature ordering that blocks shipping belongs here and in the blocking feature's plan.

<!-- /include-when-material -->

<!-- include-when-material: Spec vs Implementation — omit when spec and repo are aligned -->

## Spec vs Implementation

| Gap | Feature / unit | Notes |
|---|---|---|
| {Known drift} | {XX}n | {One line} |

Honest inventory of spec-ahead-of-code. Close via feature units; shrink this table over time.

<!-- /include-when-material -->

<!-- include-when-material: Current Focus — omit when idle between milestones -->

## Current Focus

{Active milestone, active feature, next human gate. Keep to 2–3 sentences; bump `updated:` when it changes.}

<!-- /include-when-material -->
