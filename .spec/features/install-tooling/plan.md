---
type: feature-plan
feature: install-tooling
sibling: tech.md
parent: ../../plan.md
updated: 2026-07-03
---

# Feature: Install Tooling — Implementation Plan

Refactor install.sh to an action list first (enables dry-run and uninstall as
views over the same data), then flags, then doctor + manifest on top.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** Starts when monorepo-split is `DONE` (root [plan.md](../../plan.md) Feature Sequence).

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | Partial install | install-tooling/2 |
| R2 | Dry run | install-tooling/1 |
| R3 | Uninstall | install-tooling/3 |
| R4 | Doctor | install-tooling/4 |
| R5 | Dependency manifest | install-tooling/4 |

---

## Key Technical Decisions

1. **Actions-as-data refactor first** — dry-run/uninstall become executors over one list; no parallel logic to drift.
2. **Reuse marker + cursor safety code** — both lessons.md rules already have tested implementations; call them.
3. **Doctor delegates** — `validate-state.sh`, `check-skills.sh` already exist; doctor orchestrates, doesn't duplicate.

---

### install-tooling/1 — Action-list refactor + --dry-run

**Goal:** install.sh internally builds an action list; `--dry-run` prints it and writes nothing; default behavior byte-identical to today.

**Requirements:** R2

**Dependencies:** —

**Files:**

```
install.sh  tests/adapters/run.sh
```

**Test scenarios:**

- Existing 41 adapter assertions stay green (behavior unchanged)
- `--dry-run` on fresh + installed target: non-empty plan, target tree hash unchanged

**Verification:** `bash tests/adapters/run.sh` green with new assertions.

---

### install-tooling/2 — --only spec|flow

**Goal:** partial install per half; default both; invalid value usage-errors.

**Requirements:** R1

**Dependencies:** install-tooling/1

**Files:**

```
install.sh  tests/adapters/run.sh
```

**Test scenarios:**

- `--only spec`: spec skill present, no flow/adapter/plugin files
- `--only flow`: flow + adapter present, no spec skill
- `--only bogus`: exit 1 + usage

**Verification:** adapter suite green.

---

### install-tooling/3 — --uninstall

**Goal:** remove managed artifacts; preserve `.spec/`, user AGENTS.md prose; cursor removal gated on confirm/`--yes`; composes with `--only` and `--dry-run`.

**Requirements:** R3

**Dependencies:** install-tooling/1, install-tooling/2

**Files:**

```
install.sh  flow/scripts/merge-agents.sh (reuse only)  tests/adapters/run.sh
```

**Test scenarios:**

- Install → edit user prose → uninstall → prose intact, managed gone, `.spec/` intact
- Live cursor + no `--yes` → cursor survives
- Reversed-marker AGENTS.md → refuse, byte-untouched (marker lesson regression)

**Verification:** adapter suite green.

---

### install-tooling/4 — doctor.sh + deps.json

**Goal:** one-command health report; manifest as single dep source.

**Requirements:** R4, R5

**Dependencies:** —

**Files:**

```
flow/scripts/doctor.sh  flow/reference/deps.json  tests/flow/run.sh
```

**Test scenarios:**

- Healthy source repo: all `ok` lines, exit 0
- Broken symlink / missing dep / illegal state: matching `warn` lines, still exit 0
- deps.json parses; every entry has name/kind/source/required_by/degrade

**Verification:** `bash tests/flow/run.sh` green with doctor assertions.

---

## Progress

| Unit | Status |
|---|---|
| install-tooling/1 | NOT STARTED |
| install-tooling/2 | NOT STARTED |
| install-tooling/3 | NOT STARTED |
| install-tooling/4 | NOT STARTED |
