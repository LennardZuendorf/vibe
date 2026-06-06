---
type: feature-plan
feature: agent-instructions
sibling: tech.md
parent: ../../plan.md
covers: vibe-setup AGENTS.md template merge + optional adapter symlinks
updated: 2026-06-06
---

# Feature: Agent Instructions — Implementation Plan

Ship template-driven `AGENTS.md` provisioning and optional adapter symlinks as
part of `vibe-setup` `setup.apply`. Supersedes the constitution-block merge path.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)
**Related:** [../platform-adapters/plan.md](../platform-adapters/plan.md),
[../vibe-flow/plan.md](../vibe-flow/plan.md)

Unit IDs (`AI1`…) are stable (D9).

---

## Validation Summary

**Already exists:**
- Canonical repo-root `AGENTS.md` engineering guide.
- `CLAUDE.md` → `AGENTS.md` symlink (dogfooded in this repo).
- `vibe-setup` skill with detect/apply and constitution-block template (legacy).
- `regen-active-rules.sh` (needs symlink-aware dedupe — tracked in platform-adapters).

**Must build:**
- Template file + adapter manifest under `vibe-setup/reference/`.
- Merge script (marker-aware, idempotent).
- Updated `setup.detect` / `setup.apply` in `vibe-setup` skill.
- Constitution → instructions marker migration path.

**Prerequisites:**
- **AI0** before AI4 on dogfood repo: wrap unmarked `AGENTS.md` in `vibe:instructions`.
- **Regen dedupe** before AI4 step 4 when symlinks exist, and before first `*.compound`
  after symlinks — `regen-active-rules.sh` must dedupe targets by resolved path.

---

## Implementation Roadmap

| Unit | Description | Depends |
|---|---|---|
| AI0 | Wrap-migrate dogfood `AGENTS.md` (unmarked Stage-1 guide → `vibe:instructions`) | — |
| AI1 | Add `reference/templates/AGENTS.md` + `reference/adapters.json` | AI0 |
| AI2 | Add `vibe-setup/scripts/merge-agents.sh` (marker merge + migration) | AI1 |
| AI3 | Update `setup.detect` audit surface in `vibe-setup` skill | AI1 |
| AI4 | Update `setup.apply`: merge template, prompt for symlinks, conditional regen | AI2, AI3, regen dedupe |
| AI5 | Dogfood five scenarios (see below) | AI4 |

---

## AI0 — Dogfood wrap migration

- Extend `merge-agents.sh` with **content-equivalent wrap**: when `AGENTS.md` has no
  `vibe:instructions` markers but normalized body matches template, inject markers
  around existing content without duplication.
- Apply to this repo's Stage-1 engineering guide.

**Done when:** live `AGENTS.md` has `vibe:instructions` markers; byte content inside
unchanged; `spec validate.sh` clean.

---

## AI1 — Template and manifest

- Copy current repo `AGENTS.md` into
  `.agents/skills/vibe-setup/reference/templates/AGENTS.md`.
- Wrap the vibe-owned body in `<!-- vibe:instructions:start/end -->` markers.
- Keep `vibe:active-rules` block below (empty digest).
- Add `reference/adapters.json` per [tech.md](tech.md).

**Done when:** template validates structurally; markers parseable; adapters.json
lists `CLAUDE.md` and `WARP.md`.

---

## AI2 — Merge script

- `bash .agents/skills/vibe-setup/scripts/merge-agents.sh [target-repo-root]`
- Behaviours: create-if-missing, replace-inside-markers, wrap-if-unmarked-equivalent
  (AI0), append-if-no-markers (warn), migrate `vibe:constitution` → `vibe:instructions`.
- Idempotent: second run with same template is no-op inside markers.

**Done when:** scripted tests cover fresh file, merge update, user preamble
preserved, legacy constitution migration.

---

## AI3 — Detect audit

- Extend `setup.detect` checklist in `vibe-setup/SKILL.md` per tech.md audit table.
- Report adapter states from `adapters.json`.

**Done when:** detect output lists AGENTS.md health and each adapter row without
writing anything.

---

## AI4 — Apply flow

Replace constitution-block step with:

1. Run `merge-agents.sh`.
2. Ask user which adapters to symlink (or accept future `--adapters` flag).
3. Run `link_adapter` per selection (skip existing correct symlinks; confirm before
   clobbering real files).
4. Run `regen-active-rules.sh` to seed active-rules block **only if** symlink dedupe
   is landed; otherwise seed active-rules on `AGENTS.md` only and skip adapter targets.

**Done when:** `setup.apply` on this repo is a no-op after AI0 wrap; on a fresh
sandbox creates `AGENTS.md` and optional symlinks; never overwrites user preamble;
`CLAUDE.md` symlink survives regen.

---

## AI5 — Dogfood

| # | Scenario | Verifies |
|---|---|---|
| 1 | Fresh repo, no `AGENTS.md` | AI1 create, AI4 symlink opt-in |
| 2 | Existing repo with user preamble above markers | R2 preserve outside markers |
| 3 | Legacy `vibe:constitution` block | migration to `vibe:instructions` |
| 4 | Unmarked dogfood guide (this repo) | AI0 wrap, no duplicate body |
| 5 | Adapter target is real file / stale block diff declined | R5, R10, product UX |

**Done when:** five scenarios documented; `spec validate.sh` clean.

---

## Progress

| Unit | Status |
|---|---|
| AI0 | NOT STARTED |
| AI1 | NOT STARTED |
| AI2 | NOT STARTED |
| AI3 | NOT STARTED |
| AI4 | NOT STARTED |
| AI5 | NOT STARTED |
