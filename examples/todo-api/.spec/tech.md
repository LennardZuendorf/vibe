---
type: entrypoint
scope: technical
children:
  - features/task-crud/tech.md
updated: 2026-07-03
---

# todo-api — Technical Architecture

A Node.js/Express HTTP server backed by SQLite via `better-sqlite3`. All
feature-level file lists and endpoint contracts live under
`.spec/features/<name>/`.

---

## Design Philosophy

1. **No ORM.** Raw SQL queries keep the data layer auditable and
   dependency-free.
2. **Middleware first.** Validation runs as Express middleware before route
   handlers touch the database.
3. **Schema in one place.** The SQLite `CREATE TABLE` statement is the source
   of truth; DTOs mirror it directly.

---

## Architecture Overview

```
todo-api/
├── index.js              # entry point, server boot, global error handler
├── src/
│   ├── db.js             # SQLite open/init, query helpers
│   ├── routes/
│   │   └── tasks.js      # Express Router: all /tasks endpoints
│   └── middleware/
│       └── validate.js   # Zod schema + middleware wrapper
├── tests/
│   └── tasks.test.js     # endpoint integration tests (node:test)
└── .spec/                # design docs
```

---

## Tech Stack

**Inherited:** Node.js 20 LTS, npm.

**Added:** `express` (HTTP routing), `better-sqlite3` (synchronous SQLite),
`zod` (schema validation).

---

## State / Data Contracts

| Contract | Location | Invariant |
|---|---|---|
| Tasks table schema | `src/db.js` | `id` is auto-increment PK; `title` NOT NULL |
| Task JSON shape | `src/routes/tasks.js` | Response always includes `id`, `title`, `done`, `created_at` |
| Validation schema | `src/middleware/validate.js` | Matches task table columns; 422 on mismatch |

---

## Build Sequence

| Order | Component | Feature |
|---|---|---|
| 1 | Database init + schema | task-crud |
| 2 | REST endpoints (CRUD) | task-crud |
| 3 | Validation middleware | task-crud |

---

## Features

| Feature | Covers |
|---|---|
| **[features/task-crud/](features/task-crud/tech.md)** | DB schema, route handlers, validation middleware. |
