---
type: feature-product
feature: release-docs
sibling: tech.md
parent: ../../product.md
updated: 2026-07-03
---

# Feature: Release Docs — Product

Make vibe publicly presentable and adoptable: honest two-product README set
(umbrella + per-half), trust rails (license, CI, changelog, combined test
runner), rainbow logo, worked examples, GitHub metadata, and a stranger-test
eval that proves a fresh agent can install and use vibe from the README alone.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `README.md`, `spec/README.md`, `flow/README.md`, `LICENSE`, `CHANGELOG.md`, `.github/**`, `tests/run.sh`, `docs/**` (logo, evals), `examples/**`, gh repo metadata, plugin registration docs |
| **Does not own** | Layout (monorepo-split), installer/doctor behavior (install-tooling), skill bodies |

---

## Requirements

### Requirement: Umbrella README (R1)

The root README SHALL explain within two minutes of reading: what vibe is,
the spec/flow split, who each half is for, install per half (real commands),
external dependency reality, platform support honesty, and update story.

#### Scenario: Stranger orientation

- **Given** a stranger reads only root README top-to-bottom
- **When** they finish the first two screens
- **Then** they know which half they want and the exact command to install it

### Requirement: Per-half READMEs (R2)

`spec/README.md` and `flow/README.md` SHALL each stand alone: purpose,
quickstart, day-to-day usage, file map, degrade behavior.

### Requirement: Trust rails (R3)

Repo MUST ship MIT `LICENSE`, `CHANGELOG.md` (v0.1.0), CI running shellcheck +
all test suites + spec validation on push/PR, a combined `tests/run.sh`, and
issue templates.

#### Scenario: Green badge

- **Given** the branch is pushed
- **When** CI runs
- **Then** shellcheck, three suites, and validate.sh all pass on a clean ubuntu runner

### Requirement: Logo (R4)

Repo SHALL include a rainbow SVG wordmark (3–5 candidates, one placed in
README banner) per design.md tokens.

### Requirement: Examples (R5)

`examples/` SHALL show a realistic post-install state: a filled `.spec/` with
one small worked feature spec, referenced from the READMEs.

### Requirement: GitHub presentation (R6)

Repo description and topics SHALL be set (via gh CLI); plugin registration
documented with the actual Claude Code command; social-preview upload left as
a noted manual step.

### Requirement: Stranger eval (R7)

A sandboxed agent given ONLY the README set MUST be able to (a) install
spec-only, (b) install full, (c) run one quick flow arc. Friction findings
SHALL be captured in a dated eval report and triaged (fix or explicitly
accept) before the branch PR.

#### Scenario: Eval gates the docs

- **Given** the eval report contains a blocking friction finding
- **When** finalizing the branch
- **Then** the finding is either fixed and re-evaled, or accepted with a written reason in the report

## Non-Goals

- Publishing to a plugin marketplace (documented as future step only)
- Recorded GIF/asciinema demo (needs live terminal with user — follow-up)
- Auto-uploading social preview image (GitHub UI manual step)
