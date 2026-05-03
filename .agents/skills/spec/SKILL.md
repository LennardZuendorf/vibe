---
name: spec
description: Navigate and maintain design specs in .spec/. Use BEFORE writing code or making design decisions. Triggers: when starting features, reviewing architecture, updating documentation, validating spec consistency, or when user mentions "spec", "design doc", "product requirements", or "technical design".
user-invocable: true
argument-hint: [product|tech|plan|lessons|feature|setup|validate]
allowed-tools: Read, Bash(bash ~/.agents/skills/spec/scripts/validate.sh), Bash(bash ~/.agents/skills/spec/scripts/list-specs.sh), Bash(bash ~/.agents/skills/spec/scripts/setup.sh)
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.2"
---

# Spec System

Every project's design lives in `.spec/`. This is the single source of truth for what you're building, why, and how. You read specs before writing code. You update specs when decisions change. No exceptions.

## Why Specs Exist

Specs solve the context problem. Without them, every session starts from scratch — you guess at architecture, duplicate decisions, and drift from the original vision. Specs give you:

- **Persistent memory** — decisions survive across sessions
- **Separation of concerns** — product thinking stays separate from implementation thinking
- **Progressive disclosure** — load only what you need, when you need it
- **Shared language** — consistent vocabulary between product and tech

## The Two-Layer Model

Specs come in two layers. Use the right one for the job — mixing them is the most common mistake.

```
.spec/
│
│  ─── ROOT LAYER (high-level, persistent) ──────────────
│
├── product.md                # mini PRD — story, requirements, principles
├── tech.md                   # architecture summary — stack, principles, basic implementation
├── plan.md                   # implementation roadmap
├── lessons.md                # accumulated mistakes, read at session start
├── product-{topic}.md        # cross-cutting product branch (design system, conventions)
├── tech-{topic}.md           # cross-cutting tech branch (infrastructure, observability)
│
│  ─── FEATURE LAYER (detailed, ephemeral) ──────────────
│
├── features/<name>/
│   ├── product.md            # what this feature does (requirements)
│   ├── tech.md               # how this feature is built (architecture)
│   ├── plan.md               # optional, feature-scoped roadmap
│   ├── design.md             # optional, design-system fragment
│   └── research.md           # optional, discovery artifacts
│
└── archive/<name>/           # post-merge feature specs, kept for history
```

**Root layer rules:**
- `product.md` is the mini PRD: story, target user, requirements, design principles, non-goals. Never feature-level detail.
- `tech.md` is the architecture summary: design philosophy, stack, file layout, state contracts, basic implementation, build sequence, risks. Never feature-level detail.
- Branch docs (`product-{topic}.md`, `tech-{topic}.md`) cover **cross-cutting concerns only** — things that span every feature. Design system. Infrastructure. Naming conventions. If the topic is really about one feature, it belongs in the feature layer.

**Feature layer rules:**
- One directory per feature in `.spec/features/<name>/`. Always contains `product.md` and `tech.md`. Optionally `plan.md`, `design.md`, `research.md`.
- Feature specs are **short-lived**: written during DESIGN, consumed during IMPL, merged into root layer during COMPOUND, then moved to `archive/<name>/`.
- Cross-cutting decisions from a feature get merged into root `tech.md` or relevant branch doc. Feature-specific detail does not — it stays in the archive.

## Decision: Feature vs Branch Doc

When you have content that doesn't fit in `product.md`/`tech.md`, ask one question:

**"Does this describe one feature, or something that spans every feature?"**

| Answer | Where it goes |
|---|---|
| One feature (a buildable, named unit of work) | `.spec/features/<name>/{product,tech}.md` |
| Spans every feature (design system, infra, conventions, observability) | `.spec/{product,tech}-{topic}.md` |

If unsure, default to feature. Cross-cutting concerns reveal themselves by recurring across features; you'll notice when it's time to extract them to a branch doc.

## File Type Reference

| Type | File | Purpose | Lifetime |
|------|------|---------|----------|
| Product entrypoint | `product.md` | Mini PRD — story, requirements, principles | Persistent |
| Tech entrypoint | `tech.md` | Architecture summary — stack, principles, basic impl | Persistent |
| Plan entrypoint | `plan.md` | Roadmap, milestones, progress | Persistent |
| Lessons | `lessons.md` | Mistakes and rules. Read at session start | Persistent, append-only |
| Product branch (cross-cutting) | `product-{topic}.md` | Design system, conventions | Persistent |
| Tech branch (cross-cutting) | `tech-{topic}.md` | Infrastructure, observability, deployment | Persistent |
| Sub-plan | `plan-{topic}.md` | Scoped roadmap when main plan grows large | Persistent |
| Feature product | `features/<name>/product.md` | What this feature does | Ephemeral |
| Feature tech | `features/<name>/tech.md` | How this feature is built | Ephemeral |
| Feature design | `features/<name>/design.md` | Optional UI/UX detail for a feature | Ephemeral |
| Feature research | `features/<name>/research.md` | Optional discovery artifacts | Ephemeral |
| Archive | `archive/<name>/...` | Post-merge feature specs | Frozen |

## Spec Writing Workflow

```
Step 1: product.md           — story / requirements / principles. Stay high-level.
        tech.md              — architecture / stack / basic implementation. Stay high-level.

Step 2: For each sub-part:
        features/<name>/product.md    — feature requirements
        features/<name>/tech.md       — feature architecture

Step 3: Cross-cutting only — when you notice a concern spans every feature:
        product-{topic}.md   — extract design-system, conventions, etc.
        tech-{topic}.md      — extract infrastructure, observability, etc.

Step 4: plan.md              — sequence the features into milestones
```

**Order matters:** root specs first (they constrain everything), then features. Branch docs come last and only when the cross-cutting concern is real, not anticipated.

## Navigation Rules

1. **Read entrypoints first.** Branch docs and feature specs assume you have parent context.
2. **Read lessons at session start.** Past mistakes inform current decisions.
3. **Never load all specs at once.** Load only what's relevant. Specs are designed for progressive disclosure.
4. **Follow the links.** Entrypoints link to features and branches. Trust the graph.

## Strict Rules

1. **Read before write.** Never edit a spec you haven't read in this session.
2. **Root specs stay high-level.** No feature-level detail in `product.md` or `tech.md`. If you're tempted, create a feature.
3. **One concern per doc.** Product specs contain zero code. Tech specs contain zero UX opinions. Design-system docs may cross the line — they're the only exception.
4. **Bump `updated:`.** Change the `updated:` date every time you edit a spec.
5. **Keep cross-references alive.** Link parent ↔ child both ways. List children in entrypoint frontmatter.
6. **Validate after changes.** Run `bash ~/.agents/skills/spec/scripts/validate.sh` before you're done.
7. **Feature specs are ephemeral.** Don't write them as if they're permanent. Cross-cutting decisions merge to root; feature-specific detail goes to archive.

## Product vs Tech — The Hard Line

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

The only sanctioned exception: design-system docs. Design tokens, component patterns, and visual language are inherently cross-cutting. `#00b054` is simultaneously brand identity (product) and a hex value (tech). Files with `design` scope may contain both.

## Feature Spec Lifecycle

Features are the unit of work. They follow a deliberate lifecycle.

```
Created  →  Consumed  →  Merged  →  Archived
   ↑           ↑           ↑           ↑
 DESIGN      IMPL       COMPOUND     COMPOUND
 phase       phase      phase        phase
```

1. **Created during DESIGN.** A feature spec is born when you start scoping a feature. `product.md` captures requirements; `tech.md` captures architecture decisions specific to that feature.
2. **Consumed during IMPL.** Implementation reads the feature spec, doesn't rewrite it. If reality diverges enough, the spec is amended (targeted fix), not rewritten.
3. **Merged during COMPOUND.** Cross-cutting decisions from `features/<name>/tech.md` get merged into root `tech.md`. Feature-specific detail does not.
4. **Archived after merge.** `mv .spec/features/<name>/ .spec/archive/<name>/`. Kept for history, not loaded by default.

If you don't have a `/code:feature`-style workflow, the lifecycle still applies: create the feature folder when scoping, archive it when done.

## Plans and Sub-Plans

`plan.md` is the project-level roadmap. It sequences features and milestones. It does NOT contain feature-level task lists — those go in `features/<name>/plan.md` (optional) or directly in the feature's `tech.md`.

`plan-{topic}.md` is for genuinely cross-cutting plans that don't belong to one feature (e.g., a migration that touches every feature). Rare. Default to feature-scoped plans.

## Lessons

`lessons.md` is the self-improvement log. Mistakes made during implementation, captured so they aren't repeated. Append-only. Read at session start. **Never edit during implementation** — capture mid-flight thoughts elsewhere; promote them to lessons during the COMPOUND phase, when the lesson has cooled enough to phrase in terms of pattern + rule.

### Format

```markdown
### [Short description of the mistake]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this from happening again
**Date:** YYYY-MM-DD
```

**Be specific.** "Be more careful" is not a lesson. "Always use NavigationState for routing; only create custom atoms for state NavigationState doesn't handle." is.

**Prune when stale.** If a lesson references code that no longer exists, delete it.

## Routing

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `product` | Load `.spec/product.md` and follow links |
| `tech` | Load `.spec/tech.md` and follow links |
| `plan` | Load `.spec/plan.md` and relevant sub-plans |
| `feature <name>` | Load `.spec/features/<name>/` |
| `lessons` | Load `.spec/lessons.md` |
| `setup` | Run `setup.sh` |
| `validate` | Run `validate.sh` |
| _(none)_ | Infer from task context (see table below) |

**Route by task type:**

| Task | Load First |
|---|---|
| New feature scoping | `product.md` + `tech.md`, then create `features/<name>/` |
| Working on existing feature | `features/<name>/product.md` + `features/<name>/tech.md` |
| Architecture, conventions, infrastructure | `tech.md` + relevant `tech-{topic}.md` |
| Design system, UX patterns | `product.md` + `product-{topic}.md` |
| Implementation planning, milestones | `plan.md` |
| UI / layout / user flows in a feature | `features/<name>/product.md` + `features/<name>/design.md` |

## Current Project State

!`bash ~/.agents/skills/spec/scripts/list-specs.sh`

## Setup, Templates, Validation

### Setup

`/spec setup` or `bash ~/.agents/skills/spec/scripts/setup.sh` — initializes `.spec/` with entrypoint templates and an empty `lessons.md`. Does not create features (those are born when you scope a feature).

### Templates

| File | Use for |
|---|---|
| `templates/product.md` | Root entrypoint |
| `templates/tech.md` | Root entrypoint |
| `templates/plan.md` | Project roadmap |
| `templates/product-xxx.md` | Cross-cutting product branch doc |
| `templates/tech-xxx.md` | Cross-cutting tech branch doc |
| `templates/plan-xxx.md` | Cross-cutting sub-plan |
| `templates/feature-product.md` | Feature requirements |
| `templates/feature-tech.md` | Feature architecture |

### Detailed Writing Guides

- **Product specs:** [reference/product.md](reference/product.md) — root vs feature, cross-cutting branches
- **Tech specs:** [reference/tech.md](reference/tech.md) — root vs feature, cross-cutting branches
- **Plans:** [reference/plan.md](reference/plan.md)

### Validation

`bash ~/.agents/skills/spec/scripts/validate.sh` — checks frontmatter, naming, internal links, orphaned children, and feature-folder consistency.
