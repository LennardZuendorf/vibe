# Feature layer — ephemeral per-feature specs

Each **named** unit of work gets `.spec/features/<name>/`. Specs here are **short-lived**: created in DESIGN, read in IMPL, merged in COMPOUND, then the whole folder moves to `archive/<name>/`. Global, long-living rules stay in root files — see [strategy.md](strategy.md).

## Examples in this repo

- [.spec/features/vibe-flow/](../../../.spec/features/vibe-flow) — flow state machine, `vibe-*` skills (units `VF*`)
- [.spec/features/spec-framework/](../../../.spec/features/spec-framework) — the `.spec/` planning model (units `SF*`)
- [.spec/features/agent-instructions/](../../../.spec/features/agent-instructions) — `AGENTS.md` template and adapter symlinks (units `AI*`)
- [.spec/features/platform-adapters/](../../../.spec/features/platform-adapters) — Codex / Claude Code adapters (units `U*`)

## Anatomy of `features/<name>/`

| File | Required | Purpose |
|---|---|---|
| `product.md` | yes | What this feature does (requirements, UX) |
| `tech.md` | yes | How it is built (paths, contracts, implementation) |
| `design.md` | optional | UI/UX or design-system fragment for this feature |
| `plan.md` | recommended | Stable unit IDs, dependencies, verification — see [reference/plan.md](reference/plan.md) |
| `research.md` | optional | Discovery artifacts |

**Frontmatter** (shape — adjust paths to your tree):

```yaml
---
type: feature-product   # or feature-tech
feature: <name>
sibling: tech.md          # or product.md
parent: ../../product.md # or ../../tech.md
updated: YYYY-MM-DD
---
```

Full conventions: [reference/product.md](reference/product.md) § feature product, [reference/tech.md](reference/tech.md) § feature tech.

## Lifecycle

```
Created  →  Consumed  →  Merged  →  Archived
   ↑           ↑           ↑           ↑
 DESIGN      IMPL       COMPOUND     COMPOUND
 phase       phase      phase        phase
```

1. **Created during DESIGN.** `product.md` = requirements + **Scope** table; `tech.md` = architecture for this feature only.
2. **Planned before IMPL.** `plan.md` = unit table with stable IDs (`{PREFIX}{N}`); prefix registered in root `plan.md`. Human gate on units.
3. **Consumed during IMPL.** Read feature specs; cite unit IDs in commits and tests; amend with targeted fixes if reality diverges.
4. **Verified against plan.** Evidence checked per unit verification table — not agent assertions alone.
5. **Merged during COMPOUND.** Cross-cutting blocks from `features/<name>/tech.md` promote into root `tech.md` (or branch docs). Feature-only detail does not promote.
6. **Archived after merge.** `mv .spec/features/<name>/ .spec/archive/<name>/`. Kept for archaeology, not loaded by default.

No `/code:feature` workflow? Same lifecycle: create folder when scoping, archive when done.

## Marking content for promotion (tech)

Wrap **cross-cutting** sections so COMPOUND tooling (e.g. `bin/merge-feature.sh` in your project) can promote them into root `tech.md`:

```markdown
<!-- merge -->
## Single routing contract
All `vibe-*` workflow skills and adapters call the same flow reader — one JSON shape everywhere.
<!-- /merge -->
```

Feature-only sections (one file's line count, one hook's exit code table) stay **outside** merge markers — they stay in the archive after move.

Details: [reference/tech.md](reference/tech.md) (merge markers, feature `tech.md` sections).

## What stays vs what graduates

| Stays in `features/<name>/` (archive) | Graduates to root / branch |
|---|---|
| Per-hook matcher strings, per-command LOC estimates | "Every hook delegates to keystone" |
| Feature file list for this build | Global file layout diagram |
| One feature's user-visible copy | Reusable design principles in root `product.md` |

If everything you wrote applies to **every** future feature, that content belongs in [strategy.md](strategy.md) / root branch docs — move it up, don't bury it in a feature folder.

## Scaffold checklist (new feature)

1. Create `.spec/features/<name>/`.
2. Copy templates → `product.md`, `tech.md`, `plan.md` ([feature-product](reference/templates/feature-product.md), [feature-tech](reference/templates/feature-tech.md), [feature-plan](reference/templates/feature-plan.md)).
3. Set frontmatter (`feature`, `parent`, `sibling`, `updated`) in all three files.
4. Fill **Scope** (Owns / Does not own) in `product.md`.
5. Choose unit prefix; write units in `plan.md`; register prefix in root `plan.md` feature table and unit-prefix registry.
6. Link **product ↔ tech ↔ plan** inside the folder; add standard headers (`Parent`, `Architecture`, `Plan`, `Related`).
7. Link **root** `product.md`, `tech.md`, and `plan.md` to this feature (bump root `updated:`).
8. Optional: [reference/templates/design.md](reference/templates/design.md) → `design.md` if UI-heavy.
9. Run `bash .agents/skills/spec/scripts/validate.sh` when vendored, or the equivalent global install path — [scripts/validate.sh](scripts/validate.sh).

## Archive vs delete

**Move** `features/<name>/` to `archive/<name>/`, do not delete. History preserves decisions, diffs, and "why we didn't" for later sessions.

## Templates and validation

- Feature templates: [feature-product](reference/templates/feature-product.md), [feature-tech](reference/templates/feature-tech.md), [feature-plan](reference/templates/feature-plan.md), optional [design](reference/templates/design.md)
- Writing guides: [product](reference/product.md), [tech](reference/tech.md), [plan](reference/plan.md)

Global layer handoff: [strategy.md](strategy.md).
