---
type: feature-plan
feature: task-crud
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-03
---

# Feature: Task CRUD — Implementation Plan

Deliver in three slices: schema first, then routes that depend on it, then
validation middleware that layers on top of routes. Each slice is independently
testable before the next begins.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** First and only feature — no upstream dependency.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | CRUD Operations | task-crud/1, task-crud/2 |
| R2 | Input Validation | task-crud/2, task-crud/3 |

---

## Key Technical Decisions

1. **Schema migration at startup.** `CREATE TABLE IF NOT EXISTS` means no
   migration tool for v1; add a proper migrator only when column changes are needed.
2. **Zod in middleware, not in routes.** Keeps route handlers free of validation
   branching and lets schemas be reused across create/update endpoints.

---

### task-crud/1 — Data model

**Goal:** Create the SQLite schema and `db.js` helper module.

**Requirements:** R1

**Dependencies:** —

**Files:**

```
src/db.js     # open/create tasks.db, run schema migration, export query helpers
```

**Test scenarios:**

- `db.js` loads without error on a fresh filesystem
- `SELECT * FROM tasks` returns an empty array on a new database
- Importing `db.js` twice returns the same connection singleton

**Verification:** `node -e "const db = require('./src/db'); console.log(db.all())"` prints `[]` and exits 0.

---

### task-crud/2 — REST endpoints

**Goal:** Expose the five CRUD endpoints using db helpers from task-crud/1.

**Requirements:** R1, R2

**Dependencies:** task-crud/1

**Files:**

```
src/routes/tasks.js   # Express Router for all task endpoints
index.js              # mount router, register error handler, start server
```

**Test scenarios:**

- `POST /tasks` with `{ "title": "Buy milk" }` returns 201 and a full task object
- `GET /tasks` returns an array containing previously created tasks
- `GET /tasks/:id` for an unknown id returns 404
- `DELETE /tasks/:id` removes the record and returns 204

**Verification:** `node --test tests/tasks.test.js` suite green with all CRUD cases passing.

---

### task-crud/3 — Validation middleware

**Goal:** Reject malformed requests before they reach route handlers.

**Requirements:** R2

**Dependencies:** task-crud/2

**Files:**

```
src/middleware/validate.js   # validateBody(schema) middleware using zod
```

**Test scenarios:**

- `POST /tasks` with empty body returns 422 with `{ "error": "title is required" }`
- `PUT /tasks/:id` with `{ "done": "yes" }` returns 422 (wrong type for `done`)
- Valid requests still complete normally after middleware is registered

**Verification:** validation test cases in `tests/tasks.test.js` green; no existing CRUD tests regress.

---

## Progress

| Unit | Status |
|---|---|
| task-crud/1 | NOT STARTED |
| task-crud/2 | NOT STARTED |
| task-crud/3 | NOT STARTED |
