# Strategy layer — global / long-living specs

Root-layer files in `.spec/` are **persistent**. They describe the whole product and architecture at a level that does not dive into one feature's implementation detail. Feature-level detail lives under `.spec/features/<name>/` — see [feature.md](feature.md).

## What lives at root

| File | Purpose | Lifetime |
|---|---|---|
| `product.md` | Mini PRD — story, requirements, principles, features index | Persistent |
| `tech.md` | Architecture summary — stack, principles, contracts, features index | Persistent |
| `design.md` | Cross-cutting design language, UX principles, interaction conventions | Persistent |
| `plan.md` | Milestones, feature map, unit-prefix registry, critical path | Persistent |
| `lessons.md` | Mistakes and rules — read at session start | Persistent, append-only |
| `product-{topic}.md` | Cross-cutting product (design system, conventions) | Persistent |
| `tech-{topic}.md` | Cross-cutting tech (infra, observability, deployment) | Persistent |
| `plan-{topic}.md` | Cross-cutting sub-plan when main `plan.md` grows large | Persistent |

**Root `product.md` / `tech.md`:** never feature-level detail. If you're describing how one feature works, you're in the wrong file — use `features/<name>/product.md` or `tech.md`.

## Writing order

```
Step 1: product.md     — story / requirements / principles. Stay high-level.
        tech.md        — architecture / stack / basic implementation. Stay high-level.
        design.md      — shared UX/design language. Stay high-level.

Step 2: For each sub-part (parallel or sequential):
        features/<name>/product.md   (include Scope: Owns / Does not own)
        features/<name>/tech.md
        features/<name>/plan.md      (stable unit IDs; register prefix in root plan)
        features/<name>/design.md    (optional)

Step 3: Cross-cutting only — when a concern truly spans every feature:
        product-{topic}.md
        tech-{topic}.md

Step 4: plan.md        — milestones, feature table, unit-prefix registry, critical path
```

**Order matters:** root entrypoints constrain everything; then feature folders; branch docs last and only when the cross-cutting concern is **real**, not anticipated.

Templates: [product](reference/templates/product.md), [tech](reference/templates/tech.md), [plan](reference/templates/plan.md), [feature-plan](reference/templates/feature-plan.md).

## Branch doc vs feature folder

When content doesn't fit in root `product.md` / `tech.md`, ask:

**"Does this describe one feature, or something that spans every feature?"**

| Answer | Where it goes |
|---|---|
| One feature (buildable, named unit of work) | `.spec/features/<name>/{product,tech}.md` |
| Spans every feature (design system, infra, conventions, observability) | `.spec/{product,tech}-{topic}.md` |

If unsure, default to **feature**. Recurring patterns across features signal time to extract a branch doc.

A branch doc is **not** a feature with extra steps.

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

**Two-tier planning:** root `plan.md` = milestones (M0…), feature boundaries, unit-prefix registry, critical path. `features/<name>/plan.md` = unit tables (`{PREFIX}{N}`), dependencies, verification. Never duplicate unit tables in the root plan.

Unit IDs are stable — assign once, never renumber on reorder. Cite in commits and tests during `impl`. Register each feature prefix in the root plan.

`plan-{topic}.md` is for genuinely **cross-cutting** plans that don't belong to one feature (e.g. a migration touching every feature). Rare. Default to feature-scoped plans.

Guide: [reference/plan.md](reference/plan.md).

## Lessons

`lessons.md` is the self-improvement log. Mistakes from implementation, captured so they aren't repeated. Append-only. Read at session start.

**Never edit during implementation** — capture mid-flight thoughts elsewhere; promote to lessons during COMPOUND, when the lesson can be phrased as pattern + rule.

### Format

```markdown
### [Short description of the mistake]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this from happening again
**Date:** YYYY-MM-DD
```

**Be specific.** "Be more careful" is not a lesson.

**Prune when stale.** If a lesson references code that no longer exists, delete it.

## Promotion from feature layer (COMPOUND)

During **COMPOUND**, cross-cutting decisions from `features/<name>/tech.md` merge into root `tech.md` (or a branch `tech-{topic}.md`). Feature-specific detail does **not** merge — it stays in the folder when it moves to `archive/<name>/`.

**How to mark promotable blocks** in feature `tech.md`: wrap sections in `<!-- merge -->` ... `<!-- /merge -->`, or use frontmatter `merge: true` where your project's merge tooling expects it (see root `tech.md` for `merge-feature.sh` behavior if documented in your repo).

**What typically promotes:** invariants that every feature must respect (single routing contract, global state file format, "all hooks call keystone script"). **What stays archived:** file-specific exit codes, one hook's matcher list, feature-only file paths.

If you started writing something in a feature folder and it clearly applies to **every** future feature, stop — extract to a branch doc or root instead; see [feature.md](feature.md) ↔ [strategy.md](strategy.md) handoff.

## Deep dives and validation

- **Product writing:** [reference/product.md](reference/product.md)
- **Tech writing:** [reference/tech.md](reference/tech.md)
- **Plan writing:** [reference/plan.md](reference/plan.md)

After editing root or branch specs: `bash .agents/skills/spec/scripts/validate.sh` when vendored, or the equivalent global install path — [scripts/validate.sh](scripts/validate.sh).

Feature work: [feature.md](feature.md).
