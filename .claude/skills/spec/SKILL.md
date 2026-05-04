---
name: spec
description: Navigate and maintain design specs in .spec/. Use BEFORE writing code or making design decisions. Triggers: when starting features, reviewing architecture, updating documentation, validating spec consistency, or when user mentions "spec", "design doc", "product requirements", or "technical design".
user-invocable: true
argument-hint: [product|tech|plan|lessons|setup|validate]
allowed-tools: Read, Bash(bash ~/.agents/skills/spec/scripts/validate.sh), Bash(bash ~/.agents/skills/spec/scripts/list-specs.sh), Bash(bash ~/.agents/skills/spec/scripts/setup.sh)
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.1"
---

# Spec System

Every project's design lives in `.spec/`. This is the single source of truth for what you're building, why, and how. You read specs before writing code. You update specs when decisions change. No exceptions.

## Why Specs Exist

Specs solve the context problem. Without them, every session starts from scratch — you guess at architecture, duplicate decisions, and drift from the original vision. Specs give you:

- **Persistent memory** — decisions survive across sessions
- **Separation of concerns** — product thinking stays separate from implementation thinking
- **Progressive disclosure** — load only what you need, when you need it
- **Shared language** — consistent vocabulary between product and tech

## Mental Model

Think of `.spec/` as a tree with three roots:

```
.spec/
├── product.md                  # ENTRYPOINT: What & Why (zero code)
├── tech.md                     # ENTRYPOINT: How (code welcome)
├── plan.md                     # ENTRYPOINT: Implementation roadmap
├── lessons.md                  # Accumulated mistakes and rules to prevent them
├── product-{topic}.md          # Product branch docs (UX, features, flows)
├── tech-{topic}.md             # Tech branch docs (architecture, APIs, infra)
└── plan-{topic}.md             # Feature sub-plans (optional, scoped roadmaps)
```

**Six types of files:**

| Type | File | Purpose |
|------|------|---------|
| **Product entrypoint** | `product.md` | High-level: what the product is, design principles, feature map. Read first. |
| **Tech entrypoint** | `tech.md` | High-level: architecture, stack, key patterns. Read first. |
| **Plan entrypoint** | `plan.md` | Overall implementation roadmap, milestones, progress. |
| **Lessons** | `lessons.md` | Mistake patterns, corrections, and rules to prevent repeating them. |
| **Branch doc** | `product-{topic}.md`, `tech-{topic}.md` | Deep-dive into a specific area. Assumes you've read the parent entrypoint. |
| **Sub-plan** | `plan-{topic}.md` | Feature-specific implementation plan. Scoped roadmap for a complex area (3+ milestones). |

## The Spec Writing Workflow

Specs are written in a specific order. This order matters because each layer builds on the one before it.

```
Step 1:  product.md          — Define WHAT and WHY (the big picture)
           |
Step 2:  tech.md             — Define HOW (architecture, stack, patterns)
           |
Step 3:  product-{topic}.md  — Product branch docs for specific areas
         tech-{topic}.md     — Matching tech branch docs for those areas
           |                    (always write the product branch first,
           |                     then its tech counterpart)
           |
Step 4:  plan.md             — Overall implementation roadmap
           |
Step 5:  plan-{topic}.md     — Sub-plans for complex features (optional)
```

**Why this order:**
- You can't define HOW until you know WHAT
- You can't deep-dive into features until the big picture exists
- You can't plan implementation until product and tech specs are written
- Sub-plans reference both the main plan and their feature's product/tech specs

**In practice:** Steps 1-2 happen in the first session. Step 3 happens as features get detailed. Steps 4-5 happen once enough spec material exists to plan against.

## Navigation Rules

1. **Always read the entrypoint first.** Branch docs assume you have parent context. Reading a branch doc without its entrypoint leads to misunderstanding.
2. **Read lessons at session start.** Before writing any code, check `.spec/lessons.md` for the current project. Past mistakes inform current decisions.
3. **Never load all specs at once.** Load what's relevant to your current task. Specs are designed for progressive disclosure.
4. **Follow the links.** Entrypoints link to their branches. Branches cross-reference siblings. Trust the graph.

## File Naming

Entrypoints have fixed names: `product.md`, `tech.md`, `plan.md`.

Branch docs follow: `{area}-{topic}.md`

- **area**: `product`, `tech`, or `plan` (required)
- **topic**: lowercase-with-hyphens, short and semantic (required)

**Valid:**
- `product-design.md` — UI/UX design decisions
- `product-agent.md` — agent interaction model
- `tech-infrastructure.md` — infra and build setup
- `tech-agents.md` — agent-editor integration
- `plan-editor.md` — sub-plan for editor feature
- `plan-auth.md` — sub-plan for auth implementation

**Invalid:**
- `design.md` — missing area prefix
- `product_design.md` — underscores not allowed
- `Product-Design.md` — uppercase not allowed

## Current Project State

!`bash ~/.agents/skills/spec/scripts/list-specs.sh`

## Routing

**$ARGUMENTS determines behavior:**

| Argument | Action | What to Load |
|----------|--------|--------------|
| `product` | Understand product requirements | `.spec/product.md` -> relevant branch |
| `tech` | Understand technical implementation | `.spec/tech.md` -> relevant branch |
| `plan` | Review implementation roadmap | `.spec/plan.md` -> relevant sub-plan |
| `lessons` | Review past mistakes and rules | `.spec/lessons.md` |
| `setup` | Initialize `.spec/` in current project | Run `setup.sh` |
| `validate` | Check spec consistency | Run `validate.sh` |
| _(none)_ | Infer from task context | See routing table below |

**Route by task type when no argument given:**

| Task | Load First | Then |
|------|------------|------|
| UI, layout, UX, user flows | `product.md` | Relevant product branch |
| Feature scoping, priorities, requirements | `product.md` | — |
| Architecture, code patterns, infra | `tech.md` | Relevant tech branch |
| New feature (need full picture) | `product.md` + `tech.md` | Relevant branches |
| Implementation planning, milestones | `plan.md` | Sub-plans as needed |
| Feature-specific planning | `plan.md` + `plan-{topic}.md` | Product + tech for that feature |

## Strict Rules

1. **Read before write.** Never edit a spec you haven't read in this session.
2. **One concern per doc.** Product specs contain zero code, zero implementation details. Tech specs contain zero UX opinions. This separation is sacred.
3. **Bump the `updated:` date.** Change the `updated:` frontmatter field every time you edit a spec.
4. **Keep cross-references alive.** When you add a branch doc, link it from the parent entrypoint's `children:` list and Branch Documents table. When you reference a sibling, link both ways.
5. **Validate after changes.** Run `bash ~/.agents/skills/spec/scripts/validate.sh` before you're done.

## Product vs Tech — The Hard Line

**Product specs** describe WHAT the user experiences and WHY it matters:
```markdown
# GOOD (product)
Search results appear in cards showing title, two-line excerpt, and relevance score.
Users can filter by date range and content type.

# BAD (product) — this is tech leaking in
Use SearchResultCard component with props: title, excerpt, score.
Implement with React.memo for performance.
```

**Tech specs** describe HOW to build it:
```markdown
# GOOD (tech)
SearchResultCard component:
  interface Props { title: string; excerpt: string; score: number }
  Located at src/components/SearchResultCard.tsx

# BAD (tech) — this is product leaking in
The search results should feel snappy and intuitive for the user.
```

## Plans and Sub-Plans

**`plan.md`** is the overall implementation roadmap. It covers the full project: all milestones, the critical path, and progress tracking.

**`plan-{topic}.md`** is a feature-specific sub-plan. Use one when:
- A feature area has 3+ milestones of its own
- The feature spans multiple sessions and needs independent progress tracking
- Multiple people or agents might work on this feature area concurrently
- The main plan would become unwieldy if it contained all the detail

Sub-plans reference the main plan for overall sequencing but manage their own milestones and progress. They also reference the feature's product and tech specs for requirements and architecture.

**Example:** A project with `plan.md` covering 8 milestones might have `plan-editor.md` breaking down the editor milestone into its own 4 sub-milestones with detailed tasks.

## Lessons

**`lessons.md`** is the self-improvement log. It captures mistakes made during implementation so they aren't repeated. Unlike specs which describe what to build, lessons describe what NOT to do.

### When to Update

- After ANY correction from the user
- After a bug caused by a preventable mistake
- After discovering a pattern that contradicts an assumption
- After wasting time on an approach that was already tried and failed

### Format

Each lesson has three parts:

```markdown
### [Short description of the mistake]
**Pattern:** What went wrong and why
**Rule:** The concrete rule that prevents this from happening again
**Date:** When this was learned
```

**Example:**
```markdown
### Don't mix custom atoms with NavigationState
**Pattern:** Created custom Jotai atoms for view routing instead of using Craft Agents' existing NavigationState system. This caused state conflicts and required a full rewrite.
**Rule:** Always use NavigationState for routing. Only create custom atoms for state that NavigationState doesn't handle.
**Date:** 2026-02-15
```

### How to Use

1. **Review at session start.** Before writing code, read `lessons.md` for the current project. This takes 30 seconds and can save hours.
2. **Write immediately.** Don't wait until the end of a session. Capture the lesson while the mistake is fresh.
3. **Be specific.** "Be more careful" is not a lesson. "Check that an atom exists before referencing it in a component" is.
4. **Prune when stale.** If a lesson no longer applies (e.g., the code it references was removed), delete it.

## Writing and Updating Specs

### Setting Up

Run `/spec setup` or `bash ~/.agents/skills/spec/scripts/setup.sh` to initialize `.spec/` with entrypoint templates.

### Creating a New Spec

For detailed guidance on writing each type:
- **Product specs:** [reference/product.md](reference/product.md)
- **Tech specs:** [reference/tech.md](reference/tech.md)
- **Implementation plans:** [reference/plan.md](reference/plan.md)

**Quick checklist:**
1. Pick the right type (product, tech, plan)
2. Name it correctly: `{area}-{topic}.md`
3. Add frontmatter (type, parent, scope, covers, updated)
4. Write content following the type's style rules
5. Link from parent entrypoint (`children:` list + Branch Documents table)
6. Add sibling cross-references where relevant
7. Validate: `bash ~/.agents/skills/spec/scripts/validate.sh`

### Updating an Existing Spec

1. **Read the spec first** — never edit blind
2. **Identify the correct file** — feature changes go in branch docs, scope changes go in entrypoints
3. **Make the edit** — preserve existing style and structure
4. **Bump `updated:` date** in frontmatter
5. **Check cross-references** — if your change affects referenced content, update those references
6. **Validate** — run the validation script

### Resolving Open Questions

Every spec can have an "Open Questions" section. When a question is resolved:
1. Remove it from the open questions list
2. Add the decision to the relevant section in the spec
3. Check sibling docs for cross-cutting impact
4. Bump the `updated:` date

### Templates

Use these as starting points:
- **Product entrypoint:** [reference/templates/product.md](reference/templates/product.md)
- **Tech entrypoint:** [reference/templates/tech.md](reference/templates/tech.md)
- **Plan entrypoint:** [reference/templates/plan.md](reference/templates/plan.md)
- **Product branch:** [reference/templates/product-xxx.md](reference/templates/product-xxx.md)
- **Tech branch:** [reference/templates/tech-xxx.md](reference/templates/tech-xxx.md)
- **Feature sub-plan:** [reference/templates/plan-xxx.md](reference/templates/plan-xxx.md)

### Validation

Run: `bash ~/.agents/skills/spec/scripts/validate.sh`

Checks frontmatter structure, naming conventions, broken internal links, and orphaned children.
