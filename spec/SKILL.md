---
name: spec
description: |
  Maintains `.spec/` design docs across two layers: persistent root specs
  (product, tech, design, plan, lessons, branch docs) kept current with no
  backlog, and branch-scoped `features/<name>/` folders that merge to root,
  archive transiently, then delete before the branch merges.
  Use when scoping a feature, bootstrapping strategy, reviewing architecture,
  updating a spec, validating consistency, or the user mentions spec, PRD,
  design doc, tech design, feature spec, or branch doc.
user-invocable: true
argument-hint: "[strategy|feature [<name>]|interview [<name>]|promote <name>|audit|diff <name>|health|research <name>|lessons-for <tag>|product|tech|design|plan|lessons|setup|validate]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Glob
  - Grep
  - Bash
  - Agent
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: 2.0

context:
  - .agents/skills/spec/feature.md
  - .agents/skills/spec/strategy.md
  - .agents/skills/spec/reference/product.md
  - .agents/skills/spec/reference/tech.md
  - .agents/skills/spec/reference/plan.md

agents:
  - name: spec-tracer
    path: agents/spec-tracer/SKILL.md
    trigger: feature.design step 4 (HOW codebase trace)
    caveman: lite
    parallel-safe: true
  - name: spec-promoter
    path: agents/spec-promoter/SKILL.md
    trigger: feature.compound promote step
    caveman: full
  - name: spec-interviewer
    path: agents/spec-interviewer/SKILL.md
    trigger: feature.design steps 1-2 (WHAT interview)
    caveman: lite
  - name: spec-health
    path: agents/spec-health/SKILL.md
    trigger: /spec health
    caveman: full
    user-invocable: true

superpowers:
  - superpowers:brainstorming          # WHAT interview (step 2)
  - superpowers:writing-plans          # plan units (step 5)
  - code-explorer                      # HOW codebase trace (step 4)
  - code-architect                     # HOW approach sketch (step 4)
  - superpowers:verification-before-completion  # audit quality framing
  - superpowers:finishing-a-development-branch  # compound lesson narrative

outputs:
  - .spec/features/<name>/product.md
  - .spec/features/<name>/tech.md
  - .spec/features/<name>/design.md
  - .spec/features/<name>/plan.md
  - .spec/product.md            # compound only
  - .spec/tech.md               # compound only
  - .spec/lessons.md            # compound only
  - .spec/.config.yaml          # setup only

reads:
  - .spec/product.md
  - .spec/tech.md
  - .spec/design.md
  - .spec/lessons.md
  - .spec/plan.md
  - .spec/.config.yaml
  - .spec/features/<name>/

delegates:
  - role: spec-interviewer
    when: feature product.md WHAT phase (steps 1-2)
    superpowers: [superpowers:brainstorming]
  - role: spec-tracer
    when: feature tech.md HOW phase (step 4 codebase trace)
    superpowers: [code-explorer, code-architect]
  - role: spec-promoter
    when: compound (promote cross-cutting blocks to root)
    superpowers: [superpowers:finishing-a-development-branch]
  - role: spec-health
    when: audit / structural assessment (/spec health)
    superpowers: []

phases:
  - setup.apply
  - strategy.spec
  - feature.design
  - feature.plan
  - feature.compound
  - strategy.compound

caveman:
  lite: design+plan phases; Scope table + req titles; file paths; unit IDs only
  full: impl reference + verify; all sections per template
  ultra: compound receipts; all sections + evidence + traceability matrix
---

# Spec System

Every project's design lives in `.spec/`. Single source of truth for what you're building, why, and how. Read specs before writing code. Update specs when decisions change.

## Agent quick start

1. **Pick layer** — global/root work → [strategy.md](strategy.md); one named feature → [feature.md](feature.md) + `.spec/features/<name>/`.
2. **Pick route** — explicit `/spec <arg>` → [Routing § $ARGUMENTS](#routing); no arg → [Routing § Route by task type](#routing).
3. **Finish** — bump `updated:` on every edited spec; run `bash .agents/skills/spec/scripts/validate.sh` (or global install path).

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
│  ─── FEATURE LAYER (detailed, branch-scoped) ──────────────
│
├── features/<name>/
│   ├── product.md            # what this feature does (requirements)
│   ├── tech.md               # how this feature is built (architecture)
│   ├── design.md             # optional, feature UX / interaction detail
│   ├── plan.md               # optional, feature-scoped roadmap (`<name>/n` units)
│   └── research.md           # optional, discovery artifacts
│
└── archive/<name>/           # transient post-wrapup safety net (deleted before merge)
```

**Root layer rules:**
- `product.md` is the mini PRD: story, target user, requirements, design principles, non-goals.
- `tech.md` is the architecture summary: design philosophy, stack, file layout, state contracts, basic implementation, build sequence, risks.
- `design.md` is the cross-cutting design language: UX principles, interaction conventions, interface tone, and reusable design patterns. It may bridge product and implementation vocabulary when that helps design stay actionable.
- `plan.md` sequences the work at the **root** layer: feature map, Feature Sequence with binary whole-feature gates, current focus. Current-only — no long-horizon backlog. Unit-level detail (`<name>/n`) lives in `features/<name>/plan.md`. See [reference/plan.md](reference/plan.md).
- Branch docs (`product-{topic}.md`, `tech-{topic}.md`) cover **cross-cutting concerns only** — things that span every feature. Design system. Infrastructure. Naming conventions. If the topic is really about one feature, it belongs in the feature layer.

**Feature layer rules:**
- One directory per feature in `.spec/features/<name>/`. Always contains `product.md` and `tech.md`. Recommended: `plan.md` with stable unit IDs (`<name>/n`). Optionally `design.md`, `research.md`. Every feature `product.md` includes a **Scope** table (Owns / Does not own). Each feature is a closed, deliverable, testable box; cross-feature order is a whole-feature gate in the root plan, never a unit-to-unit edge.
- Feature specs are **branch-scoped**: written during DESIGN, consumed during IMPL, merged into root layer during COMPOUND, archived as a transient safety net, then **deleted before the branch merges** (agent prompts after validation). CODE IS TRUTH; archive is never read for active work (see [Wrapped-up features](#wrapped-up-features)).
- Cross-cutting decisions from a feature get merged into root `tech.md` or relevant branch doc. Feature-specific detail does not promote — it is discarded with the deleted folder.

## Strategy vs Feature — Which layer?

| You are… | Load first |
|---|---|
| Editing or bootstrapping **global** specs (root files, branch docs, lessons, writing order, product/tech/design/plan line) | [strategy.md](strategy.md) then the root `.spec/` files it points to |
| Creating or editing **one named feature** folder under `.spec/features/<name>/` | [feature.md](feature.md) then that folder's `product.md` + `tech.md` |

**Rule of thumb:** one buildable unit of work with a name → feature layer. Something that spans every feature → root branch doc (`product-{topic}.md` / `tech-{topic}.md`). If unsure, default to feature; extract to a branch doc when the same concern keeps recurring.

## Feature authoring flow

Ladder: **locate & name → interview WHAT → rigor gate → sketch HOW → plan units → skip check.**

- **product.md** — requirements with SHALL/MUST + GWT scenarios; Scope table; no code.
- **tech.md** — paths, contracts, files traced from the repo; only filled sections.
- **plan.md** — stable unit IDs citing requirement IDs; verification evidence per unit.

Full steps, rigor gate, and skip conditions: [feature.md](feature.md).

## Superpower tips

The spec skill is a **format + constraints + validation** layer. The actual authoring work at each step is best done by delegating to the right superpower with a spec constraint document injected. **Suggest these to the user proactively** — don't wait to be asked.

| Step | Suggest to user | Constraint to inject |
|---|---|---|
| Strategy — shape direction | `superpowers:brainstorming` | Root product/tech templates from [reference/templates/](reference/templates/) |
| Feature WHAT interview | `superpowers:brainstorming` | [feature.md § Interview for WHAT](feature.md) |
| Feature HOW tracing | `code-explorer` + `code-architect` | [reference/tech.md](reference/tech.md) § feature tech |
| Feature plan units | `superpowers:writing-plans` | [reference/plan.md](reference/plan.md) + stable-ID rules |
| Compound / wrap-up | Follow [SKILL.md § Wrapped-up features](SKILL.md) — spec skill owns this sequence | — |

**How to suggest:** at the start of each step, tell the user which superpower fits and offer to delegate. Example:

> *"I can use `superpowers:writing-plans` for this step — it's purpose-built for decomposing requirements into implementable units. I'll inject the spec plan format as a constraint so the output is ready to commit. Want me to do that?"*

If the user says no, proceed with the spec skill's own guidance in [feature.md](feature.md) and [strategy.md](strategy.md). Never require superpowers — they enhance, they don't gate.

## Navigation Rules

1. **Read entrypoints first.** Branch docs and feature specs assume you have parent context.
2. **Read lessons at session start.** Past mistakes inform current decisions.
3. **Never load all specs at once.** Load only what's relevant.
4. **Follow the links.** Entrypoints link to features and branches. Trust the graph.

## Strict Rules

1. **Read before write.** Never edit a spec you haven't read in this session.
2. **Root specs stay high-level.** No feature-level detail in root `product.md` or `tech.md` — use `features/<name>/`. (Canonical statement; do not duplicate elsewhere.)
3. **One concern per doc.** Product specs contain zero code. Tech specs contain zero UX opinions. Design-system docs may cross the line — they're the only exception.
4. **Bump `updated:`.** Change the `updated:` date every time you edit a spec.
5. **Keep cross-references alive.** Link parent ↔ child both ways. List children in entrypoint frontmatter.
6. **Validate after changes.** Run `bash .agents/skills/spec/scripts/validate.sh` when the skill is vendored, or the equivalent global install path.
7. **Feature specs are branch-scoped.** Don't write them as if they're permanent. Cross-cutting decisions merge to root; the feature folder is archived transiently then deleted before the branch merges (see below). No backlog in any spec — work-ready items only; long-term ideas live in an external tracker.

## Wrapped-up features

When a feature arc completes (COMPOUND / wrap-up), follow this sequence:

1. **Promote** — merge cross-cutting blocks from `features/<name>/tech.md` (and product/design when relevant) into root `.spec/{product,tech,design,plan}.md`. Use `<!-- merge -->` markers or hand-merge; see [reference/tech.md](reference/tech.md).
2. **Record** — append a tagged lesson to `.spec/lessons.md` when a durable rule surfaced (via `vibe` compound).
3. **Update root plan** — set the feature to **DONE** in the `.spec/plan.md` Feature Sequence and cleanse delivered detail to a one-line note. Link the **live surface** (skill path, root doc section, test suite) — not the removed feature folder.
4. **Archive transiently** — `mv features/<name>/ archive/<name>/` as a safety net (e.g. CI fails right after wrapup). Archive is **never** read for active work — CODE IS TRUTH.
5. **Prompt to delete** — after validation passes, prompt the user to delete `archive/<name>/`. The folder should be gone **before the branch merges**; keeping it is the justified exception, not the default.

**Why delete and not hoard:** the repo holds value-prop + architecture + current plan only. Decision archaeology that has standalone value can be kept in archive deliberately; otherwise delete — live artifacts (skill bundle, root specs, tests) are the truth.

**This repo:** `spec-framework` was **deleted, not archived** — truth lives in this skill bundle, root `.spec/` entrypoints, and `tests/spec/run.sh`.

**Agent rules after wrap-up:**

- Route to root `plan.md` Feature Sequence + live implementation paths — **never** restore `features/<name>/` because validation failed on a wrapped-up feature.
- Do not load `archive/<name>/` by default; it is cold, transient storage when it exists.
- Full lifecycle steps: [feature.md](feature.md) § Lifecycle and § Archive and delete.

## Roles

Four subagents live under [agents/](agents/) with their own `SKILL.md`. Invoke via `/spec <role>`
or by phase-routing below. Each names an executor and the constraint document to inject — the
subagent is thin wiring between spec constraints and the appropriate superpowers executor.

| Role | Executor | Phase | Constraint document |
|---|---|---|---|
| spec-interviewer | `superpowers:brainstorming` | feature.design steps 1–2 (WHAT) | `feature.md § Interview for WHAT` |
| spec-tracer | `code-explorer` + `code-architect` | feature.design step 4 (HOW codebase trace) | `reference/tech.md` + feature-tech template |
| spec-promoter | `promote.sh` + `superpowers:finishing-a-development-branch` | feature.compound (promote) | `reference/tech.md` merge markers |
| spec-health | `validate.sh` (structural assessment) | any (`/spec health`, `/spec audit`) | validate.sh output |

Two steps delegate **inline**, with no subagent file: plan-unit writing routes to
`superpowers:writing-plans` with [reference/plan.md](reference/plan.md) + stable-ID rules, and
`/spec audit` runs [scripts/validate.sh](scripts/validate.sh) directly.

**Promote-first rule:** at each step, offer the executor before running anything. Example: *"I can use
`superpowers:writing-plans` here — want me to?"* If declined or unavailable, self-execute using the
constraint document as guidance. Never silently skip the offer; never block on the answer.

## Routing

**$ARGUMENTS:**

| Argument | Action |
|---|---|
| `strategy` | Load [strategy.md](strategy.md) — global / long-living root layer |
| `feature` | Load [feature.md](feature.md) — feature layer rules (no folder yet) |
| `feature <name>` | Load [feature.md](feature.md) + `.spec/features/<name>/` |
| `interview [<name>]` | Load spec-interviewer role; run steps 1–2 for `<name>` |
| `promote <name>` | Run `scripts/promote.sh <name>` via spec-promoter subagent |
| `audit` | Run [scripts/validate.sh](scripts/validate.sh) — structural audit (see spec-health for deeper assessment) |
| `diff <name>` | Run `scripts/scan-merges.sh <name>` — show pending merge blocks |
| `health` | Invoke spec-health subagent — structural assessment of `.spec/` tree |
| `research <name>` | Open `.spec/features/<name>/research.md`; suggest spec-tracer for discovery |
| `lessons-for <tag>` | Run `scripts/lessons-for.sh <tag>` |
| `product` | Load `.spec/product.md` and follow links |
| `tech` | Load `.spec/tech.md` and follow links |
| `design` | Load `.spec/design.md` and relevant design branch docs |
| `plan` | Load `.spec/plan.md` and relevant sub-plans |
| `lessons` | Load `.spec/lessons.md` |
| `setup` | Run setup interview (see `## Config` and `## Setup` below) |
| `validate` | Run [scripts/validate.sh](scripts/validate.sh) |
| _(none)_ | Infer from task context (see table below) |

**Route by task type:**

| Task | Load first |
|---|---|
| Project bootstrap, design language, conventions, infra spanning every feature | [strategy.md](strategy.md), then root `.spec/` as needed |
| New feature scoping | `product.md` + `tech.md`, then [feature.md](feature.md) + create `features/<name>/` |
| Working on a **wrapped-up** feature (DONE in root plan, no `features/<name>/`) | root `plan.md` live-surface link + implementation path (skill, tests, root docs) — do not restore feature folder |
| Working on existing feature | [feature.md](feature.md) + `features/<name>/product.md` + `features/<name>/tech.md` + `plan.md` when present |
| Architecture, conventions, infrastructure | `tech.md` + relevant `tech-{topic}.md` |
| Design system, UX patterns | `design.md` + relevant `product-{topic}.md` or `tech-{topic}.md` if needed |
| Implementation planning, feature sequence | root `plan.md` + `features/<name>/plan.md` + [reference/plan.md](reference/plan.md) |
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

## Config

The skill reads `.spec/.config.yaml` at session start on any `/spec` invocation. Absent file or
missing keys fall back to documented defaults without error.

| Key | Default | Behavior when set |
|---|---|---|
| `vibe-flow` | `false` | `true` → caveman level auto-managed by flow cursor; skip static default |
| `caveman` | `full` | `lite`/`auto` → apply that output profile by default; notify user |
| `suggest-superpowers` | `true` | `false` → suppress all "Superpower tip" callouts; self-execute every step |
| `superpowers.<key>` | `true` | `false` → don't offer that executor; route silently to self-execution |

Config read MUST happen before any offer or execution so `suggest-superpowers` suppression takes
effect from the first step. See setup interview below for how the file is written.

## Setup, Templates, Validation

### Setup

When the user runs `/spec setup`:

1. **Check for `.spec/.config.yaml`:**
   - If present: show current settings; ask "Which setting would you like to update?" → update only
     the named setting; re-confirm; write; done.
   - If absent: run the full interview below.

2. **Interview (4 questions — conversational, not a form):**

   **Q1 — Workflow orchestration:**
   "Are you using vibe-flow's feature dev skills (`vibe-feature`, `vibe-compound`, etc.) to manage
   your workflow, or will you run `/spec` commands directly? With vibe-flow, caveman level and phase
   routing are handled by the flow cursor."
   → Answer: yes (vibe-flow) | no (manual)

   **Q2 — Caveman level** (skip if Q1 = vibe-flow):
   "What level of spec detail do you want by default? `lite` (scope + req titles + unit IDs only),
   `full` (all sections per template — recommended), or `auto` (match to phase when detectable)."
   → Answer: lite | full | auto

   **Q3 — Available superpowers:**
   "Which AI skills/superpowers are available? `all`, `none`, or `custom` (I'll list each)."
   → Answer: all | none | {per-superpower booleans}

   **Q4 — Proactive suggestions:**
   "Should I suggest superpowers at each authoring step, or stay quiet and self-execute? Default: yes."
   → Answer: yes | no

3. **Show summary** and ask for confirmation before writing anything.

4. **On confirm:** write `.spec/.config.yaml`; then run `setup.sh` to create entrypoint templates.
   Report what was created.

5. **After setup:** surface next steps — `/spec strategy` or `/spec feature <name>`.

**Invariants:** `.spec/.config.yaml` MUST be written even when user accepts all defaults.
`setup.sh` MUST NOT be called until the user confirms the summary.

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
| [reference/templates/feature-design.md](reference/templates/feature-design.md) | Optional `features/<name>/design.md` |

**Branch docs** (`product-{topic}.md`, `tech-{topic}.md`, `plan-{topic}.md`): use the dedicated templates below. Set `type: product-topic` / `tech-topic` / `plan-topic` and parent/scope/covers per [reference/product.md](reference/product.md) and [reference/tech.md](reference/tech.md).

| Template | Use for |
|---|---|
| [reference/templates/product-topic.md](reference/templates/product-topic.md) | Cross-cutting product branch docs |
| [reference/templates/tech-topic.md](reference/templates/tech-topic.md) | Cross-cutting tech branch docs |
| [reference/templates/plan-topic.md](reference/templates/plan-topic.md) | Cross-cutting sub-plan docs |
| [reference/templates/research.md](reference/templates/research.md) | Feature `research.md` discovery artifacts |

### Detailed writing guides

- **Product:** [reference/product.md](reference/product.md) — root vs feature, cross-cutting branches
- **Tech:** [reference/tech.md](reference/tech.md) — root vs feature, merge markers, cross-cutting branches
- **Design:** [reference/design.md](reference/design.md) — root vs feature, tokens + prose
- **Plans:** [reference/plan.md](reference/plan.md)

### Validation

`bash .agents/skills/spec/scripts/validate.sh` when the skill is vendored in the repo, or `bash ~/.agents/skills/spec/scripts/validate.sh` when installed globally — checks frontmatter, naming, internal links, orphaned children, and feature-folder consistency under `.spec/`.
