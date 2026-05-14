# Feature layer — ephemeral per-feature specs

Each **named** unit of work gets `.spec/features/<name>/`. Specs here are **short-lived**: created in DESIGN, read in IMPL, merged in COMPOUND, then the whole folder moves to `archive/<name>/`. Global, long-living rules stay in root files — see [strategy.md](strategy.md).

## Examples in this repo

- [.spec/features/commands/](../../../.spec/features/commands) — command surface, lifecycles
- [.spec/features/hooks/](../../../.spec/features/hooks) — hook behavior, blocks
- [.spec/features/routing/](../../../.spec/features/routing) — keystone scripts, state

Post-archive example: [.spec/archive/engineering-agent/](../../../.spec/archive/engineering-agent) — history kept, not default-loaded.

## Anatomy of `features/<name>/`

| File | Required | Purpose |
|---|---|---|
| `product.md` | yes | What this feature does (requirements, UX) |
| `tech.md` | yes | How it is built (paths, contracts, implementation) |
| `design.md` | optional | UI/UX or design-system fragment for this feature |
| `plan.md` | optional | Feature-scoped roadmap |
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

1. **Created during DESIGN.** `product.md` = requirements; `tech.md` = architecture for this feature only.
2. **Consumed during IMPL.** Implementation reads the feature spec; amend with targeted fixes if reality diverges, don't wholesale rewrite.
3. **Merged during COMPOUND.** Cross-cutting blocks from `features/<name>/tech.md` promote into root `tech.md` (or branch docs). Feature-only detail does not promote.
4. **Archived after merge.** `mv .spec/features/<name>/ .spec/archive/<name>/`. Kept for archaeology, not loaded by default.

No `/code:feature` workflow? Same lifecycle: create folder when scoping, archive when done.

## Marking content for promotion (tech)

Wrap **cross-cutting** sections so COMPOUND tooling (e.g. `bin/merge-feature.sh` in your project) can promote them into root `tech.md`:

```markdown
<!-- merge -->
## Single routing contract
All `code-*` workflow skills and adapters call the same flow reader — one JSON shape everywhere.
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
2. Copy [reference/templates/feature-product.md](reference/templates/feature-product.md) → `product.md`; [reference/templates/feature-tech.md](reference/templates/feature-tech.md) → `tech.md`.
3. Set frontmatter (`feature`, `parent`, `sibling`, `updated`).
4. Link **product ↔ tech** inside the folder; link **root** `product.md` / `tech.md` features index tables to this folder (and bump root `updated:`).
5. Optional: [reference/templates/design.md](reference/templates/design.md) → `design.md` if UI-heavy.
6. Run `bash .agents/skills/spec/scripts/validate.sh` when vendored, or the equivalent global install path — [scripts/validate.sh](scripts/validate.sh).

## Archive vs delete

**Move** `features/<name>/` to `archive/<name>/`, do not delete. History preserves decisions, diffs, and "why we didn't" for later sessions.

## Templates and validation

- Feature templates: [reference/templates/feature-product.md](reference/templates/feature-product.md), [reference/templates/feature-tech.md](reference/templates/feature-tech.md), optional [reference/templates/design.md](reference/templates/design.md)
- Writing guides: [reference/product.md](reference/product.md), [reference/tech.md](reference/tech.md)

Global layer handoff: [strategy.md](strategy.md).
