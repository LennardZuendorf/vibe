# Strategy layer — global / long-living specs

Root-layer files in the spec root (`<root>`; `vibe-spec root` prints it) are **persistent in role, current in content**. They describe the whole product and architecture. They never hold one feature's implementation detail. The files persist. Keep their content current: reduce delivered work in `plan.md` to a one-line note, and keep no long-horizon backlog (an external tracker holds it). Feature-level detail lives in the feature layer: by default `docs/superpowers/specs/` and `docs/superpowers/plans/`; on a fallback trigger, `<root>/features/<feature>/` ([feature.md](feature.md)).

Config: spec reference/setup.md § Config.

Wrap-up: spec SKILL.md § Wrap-up.

## What lives at root

| File | Purpose | Lifetime |
|---|---|---|
| `product.md` | Mini PRD — story, requirements, principles, features index | Persistent |
| `tech.md` | Architecture summary — stack, principles, contracts, features index | Persistent |
| `design.md` | Cross-cutting design language, UX principles, interaction conventions | Persistent |
| `plan.md` | Feature sequence (binary gates), feature map, current focus | Persistent file, current content |
| `lessons.md` | Mistakes and rules — read at session start | Persistent, append-only |
| `product-{topic}.md` | Topic doc: cross-cutting product (design system, conventions) | Persistent |
| `tech-{topic}.md` | Topic doc: cross-cutting tech (infra, observability, deployment) | Persistent |
| `plan-{topic}.md` | Topic doc: cross-cutting sub-plan when main `plan.md` grows large | Persistent |

**Root `product.md` / `tech.md`:** stay high-level. Boundary: [SKILL.md](SKILL.md) § Layers (Rule 2).

## Writing order

```
Step 1: product.md     — story / requirements / principles. Stay high-level.
        tech.md        — architecture / stack / basic implementation. Stay high-level.
        design.md      — shared UX/design language. Stay high-level.

Step 2: For each feature (a closed, deliverable, testable box), in the feature layer —
        superpowers specs and plans by default; fallback folders per feature.md:
        features/<feature>/product.md   (include Scope: Owns / Does not own)
        features/<feature>/tech.md
        features/<feature>/plan.md      (stable `<feature>/n` units; same-feature deps only)
        features/<feature>/design.md    (optional)

Step 3: Topic docs, cross-cutting only — when a concern truly spans every feature:
        product-{topic}.md
        tech-{topic}.md

Step 4: plan.md        — feature table, Feature Sequence (binary gates), current focus
```

**Order matters.** Root entrypoints constrain everything. Feature work follows. Write topic docs last, and only for a **real** cross-cutting concern, never an anticipated one. Write windows (which state may write which file) are in the rendered `vibe:rules` block of `AGENTS.md`.

Shape the product direction with `superpowers:brainstorming` before writing root specs when it is installed and `suggest` is not false. Give it the root `product.md` and `tech.md` templates as the format constraint, and get your human partner's yes first.

Templates: [product](reference/templates/product.md), [tech](reference/templates/tech.md), [plan](reference/templates/plan.md), [feature-plan](reference/templates/feature-plan.md).

## Topic doc vs feature folder

When content does not fit in root entrypoints, apply the routing in [SKILL.md](SKILL.md) § Routing and the layer rules in § Layers. Put it in the feature layer. Extract to a topic doc only when a concern spans every feature.

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

**Two-tier planning:** root `plan.md` = feature boundaries, Feature Sequence with **binary whole-feature gates** (upstream DONE before downstream starts), current focus. `features/<feature>/plan.md` = unit tables (`<feature>/n`), **same-feature** dependencies, verification. Never duplicate unit tables in the root plan; cross-feature order lives only in the root Feature Sequence.

Unit IDs are `<feature>/n` — assigned once, never renumbered on reorder (add a new `<feature>/n` for new work). Cite in commits and tests during `impl`.

`plan-{topic}.md` is for genuinely **cross-cutting** plans that don't belong to one feature (e.g. a migration touching every feature). Rare. Prefer feature-scoped plans.

Guide: [reference/plan.md](reference/plan.md).

## Lessons

`lessons.md` is the self-improvement log. It records implementation mistakes so they do not repeat. Append only. Read it at session start.

**Never edit during implementation.** Capture mid-flight thoughts elsewhere. Promote them to lessons at wrap-up, as pattern + rule.

### Format

```markdown
### <Short description of the mistake>
**Pattern:** <What went wrong and why>
**Rule:** <The concrete rule that prevents it from happening again>
**Tags:** <tag>, <tag>
**Date:** YYYY-MM-DD
```

`**Tags:**` is a comma-separated list; `vibe-spec lessons-for <tag>` matches whole tags and `vibe-spec validate` warns (SF16) on an entry without the line. Add `**Pinned-by:** <the check that enforces the rule>` as a fifth line only when a test or gate enforces it; pinned entries sort first in the active-rules digest.

**Be specific.** "Be more careful" is not a lesson.

**Prune when stale.** Delete a lesson that references code that no longer exists.

## Promotion from feature layer (wrap-up)

At wrap-up, cross-cutting decisions from `features/<feature>/tech.md` merge into root `tech.md` (or a topic doc `tech-{topic}.md`). Feature-specific detail does **not** merge. Never read `archive/` for active work (CODE IS TRUTH).

Wrap-up: spec SKILL.md § Wrap-up.

**How to mark promotable blocks** in feature `tech.md`: wrap sections in `<!-- merge -->` … `<!-- /merge -->` on their own lines; `vibe-spec promote <feature>` appends them to root `tech.md`.

**What typically promotes:** invariants that every feature must respect (single routing contract, global state file format, "all hooks call keystone script"). **What stays in the archive:** file-specific exit codes, one hook's matcher list, feature-only file paths.

If content in a feature folder applies to **every** future feature, stop. Extract it to a topic doc or root (see the [feature.md](feature.md) ↔ [strategy.md](strategy.md) handoff).

## Deep dives and validation

- **Product writing:** [reference/product.md](reference/product.md)
- **Tech writing:** [reference/tech.md](reference/tech.md)
- **Plan writing:** [reference/plan.md](reference/plan.md)

After editing root or topic docs, run `vibe-spec validate`.

Feature work: [feature.md](feature.md).
