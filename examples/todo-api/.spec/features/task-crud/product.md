---
type: feature-product
feature: task-crud
sibling: tech.md
parent: ../../product.md
updated: 2026-07-03
---

# Feature: Task CRUD — Product

task-crud delivers the full lifecycle API for tasks: create, list, retrieve,
update, and delete. Server-side validation rejects malformed input before it
reaches the database.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `src/db.js`, `src/routes/tasks.js`, `src/middleware/validate.js`, SQLite tasks table |
| **Does not own** | Authentication, user management, rate limiting, deployment config |

---

## Requirements

### Requirement: CRUD Operations (R1)

The API SHALL expose endpoints to create, list, retrieve, update, and delete
tasks. Each operation MUST return JSON with a consistent task shape.

#### Scenario: Create a task

- **Given** a client POSTs `{ "title": "Buy milk" }` to `/tasks`
- **When** the request reaches the server
- **Then** the server returns 201 with `{ "id": 1, "title": "Buy milk", "done": false, "created_at": "..." }`

#### Scenario: Delete a task

- **Given** task id 7 exists
- **When** a client sends `DELETE /tasks/7`
- **Then** the server returns 204 and the task no longer appears in `GET /tasks`

#### Scenario: Get unknown task

- **Given** no task with id 99 exists
- **When** a client sends `GET /tasks/99`
- **Then** the server returns 404

### Requirement: Input Validation (R2)

The API MUST reject requests that omit required fields or supply values of the
wrong type, returning 422 with a body that identifies the offending field.

#### Scenario: Missing title on create

- **Given** a client POSTs `{}` to `/tasks`
- **When** validation runs
- **Then** the server returns 422 with `{ "error": "title is required" }`

#### Scenario: Wrong type on update

- **Given** a client sends `PUT /tasks/1` with body `{ "done": "yes" }`
- **When** validation runs
- **Then** the server returns 422 with `{ "error": "done must be boolean" }`

---

## Non-Goals

- Pagination (all tasks fit in one response for the target scale)
- Multi-user isolation or authentication
- Soft delete or audit log
