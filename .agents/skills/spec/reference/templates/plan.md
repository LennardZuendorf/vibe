---
type: plan
parent: product.md
children: []
updated: {YYYY-MM-DD}
---

# Implementation Plan

{One paragraph: current delivery focus, how features compose, and what milestone is active.}

**Product:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

---

## Features

| Feature | Product | Tech | Plan | Status |
|---|---|---|---|---|
| {name} | [features/{name}/product.md](features/{name}/product.md) | [features/{name}/tech.md](features/{name}/tech.md) | [features/{name}/plan.md](features/{name}/plan.md) | {planned / in progress / done} |

Add a row per active feature. Unit-level detail lives in feature `plan.md` — not here.

---

## Feature boundaries

{ASCII diagram or table: what each feature owns vs does not own. Prevents scope bleed.}

```
{feature-a}  ── owns ──>  {paths / concerns}
{feature-b}  ── owns ──>  {paths / concerns}
```

---

## Milestones

| ID | Goal | Features | Exit |
|---|---|---|---|
| M0 | {Foundation} | {names} | {Observable exit criteria} |
| M1 | {…} | {names} | {…} |

Milestones are delivery phases at the **root** layer. Subdivide parallel tracks as M4a / M4b when needed. Map feature units to milestones in each feature plan — do not duplicate unit tables here.

---

## Unit prefixes

| Prefix | Feature | Plan |
|---|---|---|
| {XX} | {name} | [features/{name}/plan.md](features/{name}/plan.md) |

Register every feature prefix once. Agents cite unit IDs (`{XX}1`) in commits and tests during `impl`.

---

## Critical path

```
{dependency chain — e.g. SF0 → VF1 → AI0 → U3}
```

Hard dependencies only. Cross-feature ordering that blocks shipping belongs here and in the blocking feature's plan.

---

## Spec vs implementation

| Gap | Feature / unit | Notes |
|---|---|---|
| {Known drift} | {XX}n | {One line} |

Honest inventory of spec-ahead-of-code. Close via feature units; shrink this table over time.

---

## Current focus

{Active milestone, active feature, next human gate.}
