---
type: feature-plan
feature: release-docs
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-03
---

# Feature: Release Docs — Implementation Plan

Rails first (license/CI/runner are inputs to badges), then content
(READMEs + logo + examples), then the stranger eval as the gate, then GitHub
metadata + PR. Eval findings loop back into content before the PR is opened.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

**Feature gate:** Starts when install-tooling is `DONE` (root [plan.md](../../plan.md) Feature Sequence).

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Umbrella README | release-docs/3 |
| R2 | Per-half READMEs | release-docs/4 |
| R3 | Trust rails | release-docs/1 |
| R4 | Logo | release-docs/2 |
| R5 | Examples | release-docs/5 |
| R6 | GitHub presentation | release-docs/7 |
| R7 | Stranger eval | release-docs/6 |

---

## Key Technical Decisions

1. **Rails before content** — badges/links in READMEs point at things that exist.
2. **Eval is a gate, not a report** — blocking findings fixed or written-off in the report before PR.
3. **Deps table rendered from deps.json** — single source (install-tooling R5).

---

### release-docs/1 — Trust rails

**Goal:** LICENSE (MIT), CHANGELOG 0.1.0, `tests/run.sh`, CI workflow (shellcheck + suites + validate), issue templates.

**Requirements:** R3

**Dependencies:** —

**Files:**

```
LICENSE  CHANGELOG.md  tests/run.sh  .github/workflows/ci.yml  .github/ISSUE_TEMPLATE/*
```

**Test scenarios:**

- `bash tests/run.sh` exits 0 locally, non-zero when any suite fails
- `shellcheck` clean across all repo .sh

**Verification:** local runner output; CI green after push.

---

### release-docs/2 — Logo candidates

**Goal:** 3–5 rainbow SVG candidates per design.md directions; candidate 1 placed as `docs/img/logo.svg`; swap note for user.

**Requirements:** R4

**Dependencies:** —

**Files:**

```
docs/img/logo.svg  docs/img/candidates/logo-*.svg
```

**Test scenarios:**

- Each SVG: valid XML, no font/external deps, < 20 KB, renders on light+dark

**Verification:** xmllint/render check + visual inspection note in report.

---

### release-docs/3 — Umbrella README

**Goal:** rewrite root README per R1: banner, split explanation, per-half quickstarts (real commands), dep table (from deps.json), platform honesty table, update story, plugin registration, badges.

**Requirements:** R1

**Dependencies:** release-docs/1, release-docs/2

**Files:**

```
README.md
```

**Test scenarios:**

- Every command in README executes against the repo as written
- Zero references to dead paths (grep for `.agents/flow`, `vibe-setup`, old layout)

**Verification:** command-by-command execution log; grep evidence.

---

### release-docs/4 — Per-half READMEs

**Goal:** `spec/README.md` refresh + `flow/README.md` new; each standalone per R2.

**Requirements:** R2

**Dependencies:** release-docs/3

**Files:**

```
spec/README.md  flow/README.md
```

**Test scenarios:**

- Same command-accuracy + dead-path checks as unit 3
- Cross-links umbrella ↔ halves resolve

**Verification:** execution log + link check.

---

### release-docs/5 — Examples

**Goal:** `examples/todo-api/.spec/` worked sample (root specs + one feature folder), referenced from READMEs.

**Requirements:** R5

**Dependencies:** release-docs/4

**Files:**

```
examples/todo-api/.spec/**
```

**Test scenarios:**

- Sample passes `validate.sh` when pointed at it

**Verification:** validate run against example tree.

---

### release-docs/6 — Stranger eval

**Goal:** sandboxed README-only agent installs spec-only + full, runs one quick arc; dated report with fix-or-accept per finding; blocking findings resolved.

**Requirements:** R7

**Dependencies:** release-docs/3, release-docs/4, release-docs/5

**Files:**

```
docs/evals/stranger-2026-07-03.md  (+ fixes in owning files)
```

**Test scenarios:**

- Both install paths succeed from README alone
- Quick arc transitions cursor + injects orders in sandbox

**Verification:** eval report; re-run after fixes shows blockers cleared.

---

### release-docs/7 — GitHub metadata + PR

**Goal:** repo description + topics via gh; social-preview manual note; branch pushed; PR with night summary; `REPORT.md` morning report at repo root (gitignored or PR-comment — decide at impl).

**Requirements:** R6

**Dependencies:** release-docs/6

**Files:**

```
(gh api)  REPORT.md
```

**Test scenarios:**

- `gh repo view` shows description/topics
- PR open, CI green

**Verification:** PR URL + CI status in report.

---

## Progress

| Unit | Status |
|---|---|
| release-docs/1 | DONE |
| release-docs/2 | DONE |
| release-docs/3 | DONE |
| release-docs/4 | DONE |
| release-docs/5 | NOT STARTED |
| release-docs/6 | NOT STARTED |
| release-docs/7 | NOT STARTED |
