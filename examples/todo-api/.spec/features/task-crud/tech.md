---
type: feature-tech
feature: task-crud
sibling: product.md
parent: ../../tech.md
updated: 2026-07-03
---

# Feature: Task CRUD — Architecture

task-crud owns the SQLite schema, the Express route handlers in
`src/routes/tasks.js`, and the Zod-backed validation middleware in
`src/middleware/validate.js`.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Plan:** [plan.md](plan.md)

---

## Files

```
src/db.js                    # SQLite open/init, CREATE TABLE, query helpers   ~60 LOC
src/routes/tasks.js          # Express Router: GET/POST /tasks, CRUD /tasks/:id  ~80 LOC
src/middleware/validate.js   # validateBody(schema) middleware using zod          ~30 LOC
tests/tasks.test.js          # node:test endpoint integration suite              ~100 LOC
```

---

## Contract / API

```
GET    /tasks           → 200  { tasks: Task[] }
POST   /tasks           → 201  Task        body: { title: string, description?: string }
GET    /tasks/:id       → 200  Task | 404
PUT    /tasks/:id       → 200  Task | 404  body: { title?: string, done?: boolean, description?: string }
DELETE /tasks/:id       → 204  | 404
```

Task shape (all responses):

```javascript
// src/routes/tasks.js
{
  id:          number,
  title:       string,
  description: string | null,
  done:        boolean,
  created_at:  string   // ISO 8601
}
```

Validation errors:

```json
{ "error": "<field> <problem>" }
```

---

## Implementation Detail

**DB init.** `src/db.js` opens (or creates) `data/tasks.db` on first require
and runs `CREATE TABLE IF NOT EXISTS tasks (...)`. The module exports a singleton
connection and a small set of named query helpers (`all`, `get`, `run`).

**Validation.** Each route that accepts a body calls `validateBody(schema)`
middleware. The middleware calls `schema.parse(req.body)`; on `ZodError` it
calls `next({ status: 422, message: humanise(err) })`.

**Error handling.** A single `app.use((err, req, res, next) => ...)` in
`index.js` maps `err.status` to the HTTP response code (default 500) and
serialises the body to JSON.

**Tests.** `tests/tasks.test.js` starts the server on a random port using an
in-memory SQLite database, exercises every endpoint and validation branch, then
tears down. Runs with `node --test`.
