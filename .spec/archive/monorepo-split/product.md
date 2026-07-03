---
type: feature-product
feature: monorepo-split
sibling: tech.md
parent: ../../product.md
updated: 2026-07-03
---

# Feature: Monorepo Split — Product

Restructure the repo so its two products — the spec framework and the flow
harness — are visible, browsable top-level folders (`spec/`, `flow/`) instead
of hidden dotdirs, without changing any runtime path contract. Includes the
overdue truth sweep: stale `.agents/flow` references, root plan drift, and the
compound of three shipped-but-never-compounded features.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `spec/` + `flow/` moves, `.agents/skills/*` symlinks, `install.sh` copy semantics, `.gitignore` cursor path, `.spec/**` reference sweep, AGENTS.md template path fixes + regen, `plugin.json` homepage, compound of `vibe-flow-collapse` / `vibe-skill-consolidation` / `spec-skill-improvements` |
| **Does not own** | README rewrite (release-docs), new installer flags / doctor (install-tooling), logo, CI |

---

## Requirements

### Requirement: Canonical split (R1)

The repo SHALL store the spec framework at `spec/` and the flow harness at
`flow/` as real top-level directories, with `git mv` history preserved.

#### Scenario: Browsable root

- **Given** a stranger opens the repo root on GitHub
- **When** they read the file list
- **Then** `spec/` and `flow/` are visible directories each containing the full product half

### Requirement: Runtime paths unchanged (R2)

`.agents/skills/spec` and `.agents/skills/vibe` MUST remain valid paths
(relative symlinks to `../../spec` and `../../flow`). Every runtime consumer
(hooks, scripts, tests, skill discovery) keeps referencing `.agents/skills/*`
— that path is the portable interface; root folders are storage.

#### Scenario: Scripts work through symlink

- **Given** the split layout
- **When** any `flow/scripts/*.sh` runs via `.agents/skills/vibe/scripts/`
- **Then** it resolves state, machine, and repo root identically to pre-split

### Requirement: Installer materializes real dirs (R3)

`install.sh` SHALL produce real directories under the target's
`.agents/skills/{spec,vibe}` (dereferenced copy, no symlinks in target), with
the live cursor preserved across re-install.

#### Scenario: Fresh install has no dangling links

- **Given** a target repo without vibe
- **When** `./install.sh <target>` runs from the split source
- **Then** target `.agents/skills/vibe` is a real dir, no symlink in the copied tree points outside the target

### Requirement: Truth sweep (R4)

`.spec/**` and the AGENTS.md template MUST contain zero `.agents/flow`
references; root `plan.md` test counts and Feature Sequence MUST match the
repo; the duplicated repo-layout line in AGENTS.md MUST be fixed in the
template and regenerated.

#### Scenario: Sweep is grep-clean

- **Given** the sweep is done
- **When** `grep -r "\.agents/flow" .spec/ AGENTS.md flow/reference/templates/` runs (excluding intentional test assertion)
- **Then** zero matches

### Requirement: Orphan features compounded (R5)

The three shipped features (`vibe-flow-collapse`, `vibe-skill-consolidation`,
`spec-skill-improvements`) SHALL be compounded per the spec lifecycle:
promote cross-cutting content, DONE rows in root plan, archive, prompt-delete.

#### Scenario: Lifecycle exercised

- **Given** compound completes
- **When** `validate.sh` runs
- **Then** no orphan feature folders remain in `features/`, root plan lists them as delivered notes

### Requirement: Metadata correct (R6)

`plugin.json` homepage MUST point at the real repo
(`https://github.com/LennardZuendorf/vibe`).

## Non-Goals

- Changing target-repo installed layout (targets keep `.agents/skills/`; no `spec/`+`flow/` pollution in user projects)
- Rewriting README content (release-docs owns it; only path accuracy if touched)
