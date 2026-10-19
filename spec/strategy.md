# Strategy layer — global / long-living specs

Root-layer files in `docs/spec/` are **persistent in role, current in content**. They describe the whole product and architecture. They never hold one feature's implementation detail. The files persist. Keep their content current: reduce delivered work in `plan.md` to a one-line note, and keep no long-horizon backlog (an external tracker holds it). Feature-level detail lives in the feature layer: by default `docs/superpowers/specs/` and `docs/superpowers/plans/`; on a fallback trigger, `docs/spec/features/<name>/` ([feature.md](feature.md)).

> **Config note:** If `docs/spec/.config.yaml` sets `suggest-superpowers: false`, skip every "Superpower tip" callout in this file. Run each step yourself. See [SKILL.md § Config](SKILL.md#config) for the full config reference.

## What lives at root

| File | Purpose | Lifetime |
|---|---|---|
| `product.md` | Mini PRD — story, requirements, principles, features index | Persistent |
| `tech.md` | Architecture summary — stack, principles, contracts, features index | Persistent |
| `design.md` | Cross-cutting design language, UX principles, interaction conventions | Persistent |
| `plan.md` | Feature sequence (binary gates), feature map, current focus | Persistent file, current content |
| `lessons.md` | Mistakes and rules — read at session start | Persistent, append-only |
| `product-{topic}.md` | Cross-cutting product (design system, conventions) | Persistent |
| `tech-{topic}.md` | Cross-cutting tech (infra, observability, deployment) | Persistent |
| `plan-{topic}.md` | Cross-cutting sub-plan when main `plan.md` grows large | Persistent |

**Root `product.md` / `tech.md`:** stay high-level. Boundary: [SKILL.md](SKILL.md) § Layers (Rule 2).

## Writing order

```
Step 1: product.md     — story / requirements / principles. Stay high-level.
        tech.md        — architecture / stack / basic implementation. Stay high-level.
        design.md      — shared UX/design language. Stay high-level.

Step 2: For each feature (a closed, deliverable, testable box):
        features/<name>/product.md   (include Scope: Owns / Does not own)
        features/<name>/tech.md
        features/<name>/plan.md      (stable `<name>/n` units; same-feature deps only)
        features/<name>/design.md    (optional)

Step 3: Cross-cutting only — when a concern truly spans every feature:
        product-{topic}.md
        tech-{topic}.md

Step 4: plan.md        — feature table, Feature Sequence (binary gates), current focus
```

**Order matters.** Root entrypoints constrain everything. Feature work follows. Write branch docs last, and only for a **real** cross-cutting concern, never an anticipated one.

> **Superpower tip (Step 1):** Before writing root specs, shape the product direction with `superpowers:brainstorming`. Inject the root `product.md` and `tech.md` templates as constraint context so the output lands in the right format. Ask the user: *"I can run a `superpowers:brainstorming` session on the product story and architecture before we write to `docs/spec/`. Proceed?"*

Templates: [product](reference/templates/product.md), [tech](reference/templates/tech.md), [plan](reference/templates/plan.md), [feature-plan](reference/templates/feature-plan.md).

## Branch doc vs feature folder

When content does not fit in root entrypoints, apply the routing in [SKILL.md](SKILL.md) § Routing and the layer rules in § Layers. Put it in the feature layer. Extract to branch docs only when a concern spans every feature.

## Product vs Tech — the hard line

**Product specs** describe WHAT the user experiences and WHY:

```markdown
# GOOD (product)
Search results appear in cards showing title, two-line excerpt, and relevance score.
Users can filter by date range and content type.

# BAD (product) — tech leaking in
Use SearchResultCard component with props: title, excerpt, score.
Implement with React.memo for performance.
```

**Tech specs** describe HOW to build it:

```markdown
# GOOD (tech)
SearchResultCard component:
  interface Props { title: string; excerpt: string; score: number }
  Located at src/components/SearchResultCard.tsx

# BAD (tech) — product leaking in
The search results should feel snappy and intuitive for the user.
```

The only sanctioned exception: **design-system** docs. Design tokens, component patterns, and visual language are inherently cross-cutting. `#00b054` is simultaneously brand identity (product) and a hex value (tech). Files with `design` scope may contain both.

Full guides: [reference/product.md](reference/product.md), [reference/tech.md](reference/tech.md).

## Plans and sub-plans

**Two-tier planning:** root `plan.md` = feature boundaries, Feature Sequence with **binary whole-feature gates** (upstream DONE before downstream starts), current focus. `features/<name>/plan.md` = unit tables (`<name>/n`), **same-feature** dependencies, verification. Never duplicate unit tables in the root plan; cross-feature order lives only in the root Feature Sequence.

Unit IDs are `<name>/n` — assigned once, never renumbered on reorder (add a new `<name>/n` for new work). Cite in commits and tests during `impl`.

`plan-{topic}.md` is for genuinely **cross-cutting** plans that don't belong to one feature (e.g. a migration touching every feature). Rare. Prefer feature-scoped plans.

Guide: [reference/plan.md](reference/plan.md).

## Lessons

`lessons.md` is the self-improvement log. It records implementation mistakes so they do not repeat. Append only. Read it at session start.

**Never edit during implementation.** Capture mid-flight thoughts elsewhere. Promote them to lessons during COMPOUND, as pattern + rule.

### Format

```markdown
### [Short description of the mistake]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this from happening again
**Date:** YYYY-MM-DD
```

**Be specific.** "Be more careful" is not a lesson.

**Prune when stale.** Delete a lesson that references code that no longer exists.

## Promotion from feature layer (COMPOUND)

During **COMPOUND**, cross-cutting decisions from `features/<name>/tech.md` merge into root `tech.md` (or a branch `tech-{topic}.md`). Feature-specific detail does **not** merge. Then move `features/<name>/` to `archive/<name>/`. After validation passes, prompt the user to delete the archive, before the branch merges. Never read `archive/` for active work (CODE IS TRUTH). Compound order: [SKILL.md](SKILL.md) § Layers (Rule 6).

> **Compound note:** The spec skill owns the compound sequence ([SKILL.md § Layers](SKILL.md), Rule 6): promote merge markers → record a lesson in the `§ Lessons` format → mark the root `plan.md` feature DONE → move the feature folder to `archive/<name>/` → run `validate.sh` → prompt the user to delete the archive. `superpowers:finishing-a-development-branch` handles only the mechanical git cleanup (final commit) after the spec work is done.

**How to mark promotable blocks** in feature `tech.md`: wrap sections in `<!-- merge -->` ... `<!-- /merge -->`, or use frontmatter `merge: true` where your project's merge tooling expects it (see root `tech.md` for `merge-feature.sh` behavior, if documented).

**What typically promotes:** invariants that every feature must respect (single routing contract, global state file format, "all hooks call keystone script"). **What stays in the archive:** file-specific exit codes, one hook's matcher list, feature-only file paths.

If content in a feature folder applies to **every** future feature, stop. Extract it to a branch doc or root (see the [feature.md](feature.md) ↔ [strategy.md](strategy.md) handoff).

## Deep dives and validation

- **Product writing:** [reference/product.md](reference/product.md)
- **Tech writing:** [reference/tech.md](reference/tech.md)
- **Plan writing:** [reference/plan.md](reference/plan.md)

After editing root or branch specs, run [`scripts/validate.sh`](scripts/validate.sh) from your project root.

Feature work: [feature.md](feature.md).
