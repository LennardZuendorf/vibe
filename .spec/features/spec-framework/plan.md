---
type: feature-plan
feature: spec-framework
sibling: tech.md
parent: ../../plan.md
covers: spec skill templates, validation, design-doc follow-up, SF0 lessons bootstrap
updated: 2026-06-06
---

# Feature: Spec Framework — Implementation Plan

Durable `.spec/` planning layer: the bundled `spec` skill, templates, setup, and
validation. Independent of the vibe flow — usable on its own.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Design:** [design.md](design.md)

Unit IDs (`SF1`…) are stable (D9).

---

## Validation Summary

**Already exists (Stage 1 complete):**
- Bundled `.agents/skills/spec/` skill with `strategy.md`, `feature.md`.
- `scripts/setup.sh`, `validate.sh`, `list-specs.sh`.
- Root and feature templates under `reference/templates/`.
- Root `.spec/{product,tech,design,plan,lessons}.md` in this repo.
- Feature-folder validation (frontmatter, links, required files).

**Must build (follow-up):**
- SF0: align `setup.sh` lessons template with R8 (`**Tags:**` field).
- `reference/design.md` writing guide.
- `list-specs.sh` surfaces `design.md` docs and feature folders.
- Optional `design.md` token validation (OPEN-5).

---

## Implementation Roadmap

| Unit | Description | Status |
|---|---|---|
| SF0 | Align `setup.sh` lessons bootstrap with R8 (`**Tags:**`, optional `**Pinned-by:**`) | NOT STARTED |
| SF1 | Add `reference/design.md` (DESIGN.md token-plus-prose guidance) | NOT STARTED |
| SF2 | Extend `list-specs.sh` to surface root + feature `design.md` | NOT STARTED |
| SF3 | Optional local `design.md` token-structure checks in `validate.sh` | NOT STARTED |
| SF4 | Decide + optionally wire `npx @google/design.md lint` (OPEN-5) | NOT STARTED |

---

## SF0 — Lessons bootstrap alignment

- Update `.agents/skills/spec/scripts/setup.sh` embedded lessons format comment to
  include `**Tags:**` (and optional `**Pinned-by:**`) matching canonical
  `.spec/lessons.md` and vibe-compound format.
- Optionally: `validate.sh` warns when `###` lesson entries lack `**Tags:**`.

**Done when:** fresh `setup.sh` output matches R8; dogfood repo lessons format unchanged.

---

## SF1 — Design writing guide

- Add `.agents/skills/spec/reference/design.md` derived from google-labs-code
  `DESIGN.md` patterns (token groups + prose sections).
- Link from `SKILL.md` and root `design.md` compatibility note.

**Done when:** guide exists; `spec validate.sh` still clean.

---

## SF2 — list-specs design surfacing

- `list-specs.sh` reports root `design.md` and per-feature `design.md` when present.
- Optionally list `.spec/features/*/` folders and their doc set.

**Done when:** output includes design docs alongside product/tech/plan; no `[unknown]` for `design.md`.

---

## SF3 — Local design token validation

- When `design.md` frontmatter contains token groups, `validate.sh` checks structure
  (no external deps).

**Done when:** a valid token doc passes; a malformed one warns.

---

## SF4 — External design linter (OPEN-5)

- Document decision in root plan OPEN-5.
- If yes: optional flag or env gate for `npx @google/design.md lint`.

**Done when:** decision recorded; implementation matches decision.

---

## Progress

| Unit | Status |
|---|---|
| SF0 | NOT STARTED |
| SF1 | NOT STARTED |
| SF2 | NOT STARTED |
| SF3 | NOT STARTED |
| SF4 | NOT STARTED |
