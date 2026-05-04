# Writing Implementation Plans

The plan is the bridge between specs and code. It takes the WHAT from `product.md` and the HOW from `tech.md` and turns them into a sequenced, validated roadmap of milestones and sessions.

## The Planning Mindset

A good plan is honest about complexity. It accounts for what already exists (don't rebuild it), what's genuinely new (estimate conservatively), and what depends on what (sequence correctly). Optimistic plans cause constant re-planning. Realistic plans let you focus on building.

Plans are living documents. They get validated against the actual codebase, updated as milestones complete, and adjusted when reality diverges from estimates.

## Structure

### `plan.md`

The plan entrypoint contains:

- **Validation summary** — what has been verified against the actual codebase (what exists, what must be built)
- **Critical architecture decisions** — decided items and items still to resolve, with their status
- **Implementation roadmap** — table of milestones with goals, session estimates, risk levels, and any changes from earlier versions
- **Milestone details** — each milestone broken into concrete tasks with validation criteria
- **Critical path** — the dependency chain that determines total timeline
- **Progress tracking** — sessions used vs estimated, current milestone status

## Key Sections Explained

### Validation Summary

Before a plan is trustworthy, it must be validated against the codebase. This section records:

```markdown
## Validation Summary

This plan has been validated against the actual codebase. Key findings:

Already exists (don't rebuild):
- Mention system (useInlineMention hook, [file:path] support)
- File operations (extensive IPC handlers)
- Theme system (CSS custom properties)

Must build:
- Markdown editor (tiptap)
- File tree UI
- Local source type implementation
```

This prevents the most common planning mistake: estimating work for things that already exist.

### Architecture Decisions

Track decisions that affect the plan's structure. Use clear status markers:

```markdown
## Critical Architecture Decisions

### Decided
- **IPC Namespace:** `notes:*` (not `file:*`)
- **Session-File Binding:** Explicit attach/detach per session

### To Resolve
- [ ] Route pattern for note views
- [ ] Auto-save algorithm details
```

### Milestone Breakdown

Each milestone needs:
- **Goal** — one sentence describing the exit state
- **Session estimate** — realistic, not optimistic
- **Risk level** — Low / Medium / High
- **Tasks** — concrete, verifiable items
- **Validation criteria** — how you know the milestone is done

```markdown
## M2: Editor Shell

**Goal:** tiptap rendering in Electron with toolbar and theming.
**Sessions:** 2-3 | **Risk:** Medium

Tasks:
- [ ] Install tiptap dependencies
- [ ] Scaffold simple-editor template
- [ ] Wire editor component into layout
- [ ] Connect theme tokens

**Done when:** Editor renders markdown, toolbar works, dark/light mode switches correctly.
```

### Critical Path

Show the dependency chain:
```markdown
## Critical Path

Pre-M1 -> M1 -> M2+M3 -> M4a -> M5 -> M5b -> M7 -> M8
```

This tells you which milestones can be parallelized and which are sequential blockers.

### Progress Tracking

Update as you go. Valid status values: `NOT STARTED`, `IN PROGRESS`, `DONE`, `BLOCKED`, `SKIPPED`.

```markdown
## Progress

| Milestone | Status | Sessions Used | Estimate |
|-----------|--------|---------------|----------|
| Pre-M1 | DONE | 0.5 | 0.5 |
| M1 | DONE | 1 | 1 |
| M2+M3 | DONE | 3 | 2-3 |
| M4a | IN PROGRESS | 1 | 1.5-2 |
```

## Style Rules

**Do:**
- Validate against the actual codebase before finalizing
- Use session counts (not hours/days) as estimates
- Mark tasks with checkboxes for tracking
- Include "changes from original" when plans evolve
- Keep milestones small enough to complete in 1-3 sessions
- State validation criteria as concrete, verifiable conditions

**Don't:**
- Plan more than one level of detail ahead (detail the next 2-3 milestones, keep future ones high-level)
- Mix product decisions into the plan — reference the product spec
- Include code in the plan — that's what tech specs are for
- Create overly optimistic estimates — add buffer for integration complexity

## Relationship to Other Specs

The plan references both product and tech specs but doesn't duplicate their content:

```markdown
**Parent specs:** [product.md](product.md), [tech.md](tech.md)
```

When a milestone's tasks require detailed understanding, the plan points to the relevant spec section rather than restating it.

## Sub-Plans: `plan-{topic}.md`

When a feature area is complex enough that its implementation details would bloat the main plan, break it out into a sub-plan. Sub-plans are to `plan.md` what branch docs are to `product.md` and `tech.md` — a scoped deep-dive.

### When to Create a Sub-Plan

Create a `plan-{topic}.md` when:
- A feature area has 3+ milestones of its own
- The feature spans multiple sessions and needs independent progress tracking
- Multiple agents might work on this area concurrently and need a clear scope
- The main plan would exceed ~200 lines if all the detail were inline

Don't create a sub-plan for:
- Simple features that fit in a single milestone in the main plan
- Work that doesn't have its own product and tech specs to reference
- Temporary task lists (those belong in your session, not in specs)

### Structure of a Sub-Plan

A sub-plan mirrors the main plan structure but scoped to one feature:

- **Parent/sibling links** — links back to `plan.md` (parent) and other sub-plans (siblings) for navigation
- **Product and tech spec references** — links to the feature's `product-{topic}.md` and `tech-{topic}.md`
- **Scope definition** — what this sub-plan covers vs what stays in the main plan
- **Pre-implementation checklist** — specs reviewed, dependencies from main plan satisfied
- **Feature-scoped milestones** — prefixed with the topic (e.g., `editor-M1`, `editor-M2`)
- **Progress tracking** — independent from the main plan's progress table

### How Sub-Plans Relate to the Main Plan

The main plan references sub-plans but doesn't duplicate their detail:

```markdown
## M5: Editor

**Goal:** Full markdown editor with file I/O.
**Sessions:** 6-8 | **Risk:** High

Detailed breakdown: [plan-editor.md](plan-editor.md)
```

The sub-plan manages its own milestones and progress, but its top-level milestone (`M5: Editor`) is tracked in the main plan's progress table.

### Naming Convention

Sub-plans follow the same `{area}-{topic}.md` pattern as other branch docs:
- `plan-editor.md` — editor implementation plan
- `plan-auth.md` — authentication feature plan
- `plan-file-sync.md` — file synchronization plan

The topic should match the corresponding product/tech branch docs when they exist (e.g., `plan-editor.md` pairs with `product-editor.md` and `tech-editor.md`).

## When to Update

- **Milestone completed** — mark as DONE, update sessions used
- **Estimate changed** — update the milestone table with revised numbers
- **Scope changed** — add/remove milestones, update critical path
- **Architecture decision resolved** — move from "To Resolve" to "Decided"
- **Plan validated** — add validation summary, adjust estimates based on findings

## Templates

See [templates/plan.md](templates/plan.md) for the main plan template and [templates/plan-xxx.md](templates/plan-xxx.md) for the feature sub-plan template.
