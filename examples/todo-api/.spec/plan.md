---
type: entrypoint
scope: implementation
covers: feature sequence, build order, validation criteria
children: []
updated: 2026-07-03
---

# todo-api — Implementation Plan

Single-feature project: deliver task-crud and the API is ready to ship. Unit
detail lives in the feature plan, not here.

**Parent specs:** [product.md](product.md), [tech.md](tech.md)

**Feature plans:**

| Feature | Product | Plan | Status |
|---|---|---|---|
| [task-crud](features/task-crud/product.md) | endpoints + validation | [plan.md](features/task-crud/plan.md) | in progress |

---

## Feature Boundaries

```
task-crud  ── owns ──>  src/db.js, src/routes/tasks.js, src/middleware/validate.js
```

| Layer | Owns | Does not own |
|---|---|---|
| **task-crud** | DB schema, CRUD routes, validation middleware | Auth, rate limiting, deployment config |

---

## Feature Sequence

| Order | Feature | Deliverable | Test | Status | Starts when |
|---:|---|---|---|---|---|
| 1 | task-crud | SQLite schema + CRUD routes + validation | `tests/tasks.test.js` | in progress | — |

---

## Current Focus

task-crud is the only feature. It delivers the full CRUD API plus input
validation in three slices (schema → routes → validation), each independently
testable.
