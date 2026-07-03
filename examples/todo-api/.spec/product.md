---
type: entrypoint
scope: product
children:
  - features/task-crud/product.md
updated: 2026-07-03
---

# todo-api — Product

todo-api is a lightweight REST API for managing personal task lists. It stores
tasks with a title, optional description, and completion status, and exposes a
JSON API over HTTP for client applications.

**One-liner:** a dead-simple, self-hosted REST API for todo lists.

---

## Story

Small projects need a task API they can own end-to-end without a BaaS account.
todo-api fills that gap: a single Node.js process with a local SQLite store,
deployable on any VPS with no external dependencies. When it ships, a developer
can `git clone` → `npm install` → `node .` and have a working API in under a
minute.

---

## Requirements

At a project level, todo-api must:

1. **Expose CRUD over tasks.** Clients can create, read, update, and delete
   tasks via standard HTTP methods and JSON payloads.
2. **Validate inputs.** The API must reject malformed requests with a clear
   error response rather than silently persisting bad data.
3. **Operate without external services.** No cloud database, queue, or auth
   provider is required to run the server.

---

## Design Principles

1. **Single process, single file of state.** SQLite keeps the runtime surface
   tiny — no daemon, no migration tool for basic use.
2. **Standard REST conventions.** HTTP status codes and JSON bodies map
   directly to spec; no proprietary envelope.
3. **Fail loudly on bad input.** 422 on validation error with a body that names
   the problem — never a silent 200.

---

## Target User

A solo developer or small team who wants a self-hosted todo backend they fully
control, with no external accounts and easy deployment via a single command.

---

## Features

| Feature | Covers |
|---|---|
| **[features/task-crud/](features/task-crud/product.md)** | Full lifecycle CRUD endpoints for tasks plus input validation. |
