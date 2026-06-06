---
name: spec
description: |
  Maintain `.spec/` design docs across two layers: long-living root specs
  (`product.md`, `tech.md`, `design.md`, `plan.md`, `lessons.md`, branch docs)
  and ephemeral per-feature folders under `features/<name>/` that merge to root
  then archive.
  Read before code. Trigger on: scoping a feature, bootstrapping strategy,
  reviewing architecture, updating a spec, validating consistency, or user says
  spec, PRD, design doc, tech design, feature spec, branch doc.
user-invocable: true
argument-hint: "[strategy|feature|product|tech|plan|lessons|setup|spec]"
allowed-tools: Read, Bash(bash .agents/skills/spec/scripts/validate.sh), Bash(bash .agents/skills/spec/scripts/list-specs.sh), Bash(bash .agents/skills/spec/scripts/setup.sh), Bash(bash ~/.agents/skills/spec/scripts/validate.sh), Bash(bash ~/.agents/skills/spec/scripts/list-specs.sh), Bash(bash ~/.agents/skills/spec/scripts/setup.sh)
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.4"
---

# Spec System

Every project's design lives in `.spec/`. Single source of truth for what you're building, why, and how. Read specs before writing code. Update specs when decisions change.

## Why Specs Exist

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
├── design.md                 # cross-cutting product/UX design language
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
│   ├── design.md             # optional, feature UX / interaction detail
│   ├── plan.md               # optional, feature-scoped roadmap
│   └── research.md           # optional, discovery artifacts
│
└── archive/<name>/           # post-merge feature specs, kept for history
```

**Root layer rules:**
- `product.md` is the mini PRD: story, target user, requirements, design principles, non-goals. Never feature-level detail.
- `tech.md` is the architecture summary: design philosophy, stack, file layout, state contracts, basic implementation, build sequence, risks. Never feature-level detail.
- `design.md` is the cross-cutting design language: UX principles, interaction conventions, interface tone, and reusable design patterns. It may bridge product and implementation vocabulary when that helps design stay actionable.
- `plan.md` sequences the work at the **root** layer: milestones, feature map, unit-prefix registry, critical path. Unit-level detail lives in `features/<name>/plan.md`. See [reference/plan.md](reference/plan.md).
- Branch docs (`product-{topic}.md`, `tech-{topic}.md`) cover **cross-cutting concerns only** — things that span every feature. Design system. Infrastructure. Naming conventions. If the topic is really about one feature, it belongs in the feature layer.

**Feature layer rules:**
- One directory per feature in `.spec/features/<name>/`. Always contains `product.md` and `tech.md`. Recommended: `plan.md` with stable unit IDs (`{PREFIX}{N}`). Optionally `design.md`, `research.md`. Every feature `product.md` includes a **Scope** table (Owns / Does not own).
- Feature specs are **short-lived**: written during DESIGN, consumed during IMPL, merged into root layer during COMPOUND, then moved to `archive/<name>/`.
- Cross-cutting decisions from a feature get merged into root `tech.md` or relevant branch doc. Feature-specific detail does not — it stays in the archive.

## Strategy vs Feature — Which layer?

| You are… | Load first |
|---|---|
| Editing or bootstrapping **global** specs (root files, branch docs, lessons, writing order, product/tech/design/plan line) | [strategy.md](strategy.md) then the root `.spec/` files it points to |
| Creating or editing **one named feature** folder under `.spec/features/<name>/` | [feature.md](feature.md) then that folder's `product.md` + `tech.md` |

**Rule of thumb:** one buildable unit of work with a name → feature layer. Something that spans every feature → root branch doc (`product-{topic}.md` / `tech-{topic}.md`). If unsure, default to feature; extract to a branch doc when the same concern keeps recurring.

## Navigation Rules

1. **Read entrypoints first.** Branch docs and feature specs assume you have parent context.
2. **Read lessons at session start.** Past mistakes inform current decisions.
3. **Never load all specs at once.** Load only what's relevant.
4. **Follow the links.** Entrypoints link to features and branches. Trust the graph.

## Strict Rules

1. **Read before write.** Never edit a spec you haven't read in this session.
2. **Root specs stay high-level.** No feature-level detail in `product.md` or `tech.md`. If you're tempted, create a feature.
3. **One concern per doc.** Product specs contain zero code. Tech specs contain zero UX opinions. Design-system docs may cross the line — they're the only exception.
4. **Bump `updated:`.** Change the `updated:` date every time you edit a spec.
5. **Keep cross-references alive.** Link parent ↔ child both ways. List children in entrypoint frontmatter.
6. **Validate after changes.** Run `bash .agents/skills/spec/scripts/validate.sh` when the skill is vendored, or the equivalent global install path.
7. **Feature specs are ephemeral.** Don't write them as if they're permanent. Cross-cutting decisions merge to root; feature-specific detail goes to archive.

## Routing

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `strategy` | Load [strategy.md](strategy.md) — global / long-living root layer |
| `feature` | Load [feature.md](feature.md) — feature layer rules (no folder yet) |
| `feature <name>` | Load [feature.md](feature.md) + `.spec/features/<name>/` |
| `product` | Load `.spec/product.md` and follow links |
| `tech` | Load `.spec/tech.md` and follow links |
| `design` | Load `.spec/design.md` and relevant design branch docs |
| `plan` | Load `.spec/plan.md` and relevant sub-plans |
| `lessons` | Load `.spec/lessons.md` |
| `setup` | Run [scripts/setup.sh](scripts/setup.sh) |
| `validate` | Run [scripts/validate.sh](scripts/validate.sh) |
| _(none)_ | Infer from task context (see table below) |

**Route by task type:**

| Task | Load first |
|---|---|
| Project bootstrap, design language, conventions, infra spanning every feature | [strategy.md](strategy.md), then root `.spec/` as needed |
| New feature scoping | `product.md` + `tech.md`, then [feature.md](feature.md) + create `features/<name>/` |
| Working on existing feature | [feature.md](feature.md) + `features/<name>/product.md` + `features/<name>/tech.md` + `plan.md` when present |
| Architecture, conventions, infrastructure | `tech.md` + relevant `tech-{topic}.md` |
| Design system, UX patterns | `design.md` + relevant `product-{topic}.md` or `tech-{topic}.md` if needed |
| Implementation planning, milestones | root `plan.md` + `features/<name>/plan.md` + [reference/plan.md](reference/plan.md) |
| UI / layout / user flows in a feature | `features/<name>/product.md` + `features/<name>/design.md` (if present) |

## Design.md Compatibility

For visual identity or UI design systems, prefer the public
[`DESIGN.md` format](https://github.com/google-labs-code/design.md): YAML
frontmatter for machine-readable tokens plus markdown prose for human rationale.
Use it inside `.spec/design.md` or `.spec/features/<name>/design.md`; do not move
design docs out of `.spec/`.

Useful token groups:
- `colors`
- `typography`
- `rounded`
- `spacing`
- `components`

Recommended section order for visual design docs:
1. Overview
2. Colors
3. Typography
4. Layout
5. Elevation & Depth
6. Shapes
7. Components
8. Do's and Don'ts

For non-visual workflow design, the same file may omit tokens and use prose-only
sections such as interaction conventions, information hierarchy, and agent tone.

## Current Project State

!`bash .agents/skills/spec/scripts/list-specs.sh`

## Setup, Templates, Validation

### Setup

`/spec setup` or `bash .agents/skills/spec/scripts/setup.sh` from a repo that vendors this skill — initializes `.spec/` with entrypoint templates and an empty `lessons.md`. If the skill is installed globally, `bash ~/.agents/skills/spec/scripts/setup.sh` is also valid. Does not create features (those are born when you scope a feature).

### Templates

Paths are under [reference/templates/](reference/templates/) (copy into your project's `.spec/`).

| Template | Use for |
|---|---|
| [reference/templates/product.md](reference/templates/product.md) | Root `product.md` |
| [reference/templates/tech.md](reference/templates/tech.md) | Root `tech.md` |
| [reference/templates/design.md](reference/templates/design.md) | Root `design.md` or feature `design.md` |
| [reference/templates/plan.md](reference/templates/plan.md) | Root `plan.md` |
| [reference/templates/feature-product.md](reference/templates/feature-product.md) | `features/<name>/product.md` |
| [reference/templates/feature-tech.md](reference/templates/feature-tech.md) | `features/<name>/tech.md` |
| [reference/templates/feature-plan.md](reference/templates/feature-plan.md) | `features/<name>/plan.md` |
| [reference/templates/design.md](reference/templates/design.md) | Optional `features/<name>/design.md` or design-system fragment |

**Branch docs** (`product-{topic}.md`, `tech-{topic}.md`, `plan-{topic}.md`): no separate template files in this bundle — start from the root `product.md` / `tech.md` / `plan.md` templates, rename, set `type: branch` and parent/scope/covers per [reference/product.md](reference/product.md) and [reference/tech.md](reference/tech.md).

### Detailed writing guides

- **Product:** [reference/product.md](reference/product.md) — root vs feature, cross-cutting branches
- **Tech:** [reference/tech.md](reference/tech.md) — root vs feature, merge markers, cross-cutting branches
- **Plans:** [reference/plan.md](reference/plan.md)

### Validation

`bash .agents/skills/spec/scripts/validate.sh` when the skill is vendored in the repo, or `bash ~/.agents/skills/spec/scripts/validate.sh` when installed globally — checks frontmatter, naming, internal links, orphaned children, and feature-folder consistency under `.spec/`.
