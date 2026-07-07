---
type: feature-product
feature: install-tooling
sibling: tech.md
parent: ../../product.md
updated: 2026-07-03
---

# Feature: Install Tooling — Product

Give adopters a trustworthy install lifecycle: partial install (spec-only),
preview before touching anything, clean removal, a one-command health check,
and an explicit machine-readable list of external skill dependencies. Turns
"movable + copyable" into "safe to try and safe to leave."

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `install.sh` flags (`--only`, `--dry-run`, `--uninstall`), `flow/scripts/doctor.sh`, `flow/reference/deps.json`, their tests |
| **Does not own** | Layout/symlinks (monorepo-split), READMEs and docs rendering deps (release-docs), hook behavior |

---

## Requirements

### Requirement: Partial install (R1)

`install.sh <target> --only spec` SHALL install only the spec framework (spec
skill + `.spec` bootstrap pointer); `--only flow` only the flow harness +
adapter. Default remains both.

#### Scenario: Spec-only leaves no flow trace

- **Given** a clean target
- **When** `--only spec` runs
- **Then** target has `.agents/skills/spec/` and no `.agents/skills/vibe/`, no `.claude/hooks`, no plugin manifest

### Requirement: Dry run (R2)

`install.sh <target> --dry-run` MUST print every action it would take and
write nothing.

#### Scenario: Target byte-untouched

- **Given** any target state
- **When** `--dry-run` runs
- **Then** printed plan is non-empty and the target tree is byte-identical before/after

### Requirement: Uninstall (R3)

`install.sh <target> --uninstall` SHALL remove managed artifacts (copied
skills, adapter files, managed AGENTS.md block) while preserving user
content: `.spec/**`, cursor file removed only with explicit confirm, user-owned
AGENTS.md prose untouched.

#### Scenario: User content survives removal

- **Given** an installed target with user edits outside markers and a live cursor
- **When** `--uninstall` runs
- **Then** managed files are gone, AGENTS.md user prose intact, `.spec/` intact

### Requirement: Doctor (R4)

`doctor.sh` SHALL report install health in one run: core files present,
symlink/dir integrity, cursor validity, adapter wiring, external deps
presence per manifest — warn-only, always exit 0, one line per check.

#### Scenario: Degraded deps reported not fatal

- **Given** superpowers skills absent
- **When** `doctor.sh` runs
- **Then** a warn line names the missing dep + degrade consequence, exit code 0

### Requirement: Dependency manifest (R5)

External skill dependencies MUST live in one machine-readable manifest
(name, kind, source URL, required-by, degrade behavior) consumed by doctor
and renderable into docs.

#### Scenario: Single source

- **Given** the manifest
- **When** docs or doctor mention dependencies
- **Then** both derive from the same file — no second hand-written list

## Non-Goals

- Auto-installing external dependencies (superpowers etc.) — report only
- Version/update management beyond "re-run install.sh"
