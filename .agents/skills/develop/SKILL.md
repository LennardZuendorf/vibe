---
name: develop
description: Full feature development lifecycle with phase gates. Enforces research → spec → plan → implement → review. Use when starting ANY feature, bug fix, or refactoring. Triggers on "develop", "feature", "implement", "build", "new feature", "add feature".
user-invocable: true
argument-hint: <feature-description>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Agent, Skill, TodoWrite, AskUserQuestion
compatibility: Requires bash. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# Develop — Spec-Driven Feature Lifecycle

You are a senior product engineer. You do not skip steps. You research before you plan, you plan before you code, and you review before you ship.

This skill orchestrates a complete feature development lifecycle through 5 mandatory phases. Each phase has clear goals, allowed actions, and exit criteria. **You must complete each phase before moving to the next.**

## Phase State

The current phase is tracked in `.spec/.phase` as a simple text file containing one of: `RESEARCH`, `SPEC`, `PLAN`, `IMPLEMENT`, `REVIEW`, `DONE`.

Before anything else, check if `.spec/.phase` exists:
- If it exists, read it and resume from that phase
- If it doesn't exist, start at RESEARCH

Update `.spec/.phase` whenever you transition between phases.

## The Feature Request

**User asked for:** $ARGUMENTS

---

## Phase 1: RESEARCH

**Goal:** Deeply understand the problem space, existing code, and constraints before writing anything.

**You MUST do all of the following:**

1. **Read context first:**
   - Read `.spec/lessons.md` if it exists (learn from past mistakes)
   - Read `.spec/product.md` and `.spec/tech.md` entrypoints if they exist
   - Read `CLAUDE.md` if it exists

2. **Explore aggressively with subagents** — spawn multiple Explore agents in parallel:
   - Agent 1: Search for existing code related to the feature (Glob + Grep patterns)
   - Agent 2: Search for related tests, fixtures, and examples
   - Agent 3: Search for related configuration, types, and interfaces
   - Agent 4: If external APIs/libraries are involved, use WebSearch/WebFetch
   - Run all agents that make sense in **parallel** — do NOT run them sequentially

3. **Write findings to file** — Create `.spec/research.md` with a structured summary:
   - What already exists that's relevant (with file paths)
   - What needs to be built (gap analysis)
   - What constraints or patterns you discovered
   - Any risks or open questions
   - This file preserves context across compactions and lets future phases reference it

4. **Present to user** — Show your research findings and ask: "Does this match your understanding? Any corrections before I write specs?"

**Exit criteria:** User confirms research findings are accurate.

**Then:** Write `SPEC` to `.spec/.phase` and proceed.

---

## Phase 2: SPEC

**Goal:** Write product and tech specs that fully describe the feature.

**You MUST do all of the following:**

1. **Initialize if needed** — If `.spec/` doesn't exist, run `/spec setup`

2. **Write specs in order** (this order is mandatory):
   a. Update or create `product.md` / `product-{feature}.md` — describe WHAT and WHY
   b. Update or create `tech.md` / `tech-{feature}.md` — describe HOW
   c. Follow all rules from the `/spec` skill (frontmatter, naming, cross-refs)

3. **Validate** — Run `bash ~/.agents/skills/spec/scripts/validate.sh`

4. **Present to user** — Show spec summaries and ask: "Are these specs accurate? Should I adjust anything before planning?"

**Rules during this phase:**
- You may ONLY write to files inside `.spec/`
- No implementation code, no test files, no config changes
- Product specs contain ZERO code (this is sacred)
- Tech specs reference real file paths and include code examples

**Exit criteria:** Specs validated, user approves.

**Then:** Write `PLAN` to `.spec/.phase` and proceed.

---

## Phase 3: PLAN

**Goal:** Create a concrete implementation plan with milestones, tasks, and validation criteria.

**You MUST do all of the following:**

1. **Read the specs you just wrote** — load product and tech specs for context

2. **Create or update the plan:**
   - Update `.spec/plan.md` with milestones for this feature
   - For complex features (3+ milestones), create `.spec/plan-{feature}.md`
   - Include: validation summary, tasks with checkboxes, session estimates, risk levels
   - Include validation criteria for each milestone (how do you know it's done?)

3. **Set up tracking** — Create a TodoWrite list mirroring the plan's tasks

4. **Present to user** — Show the plan and ask: "Ready to implement? Any changes to the plan?"

**Rules during this phase:**
- You may ONLY write to `.spec/plan*.md` files
- No implementation code

**Exit criteria:** Plan created, user approves.

**Then:** Write `IMPLEMENT` to `.spec/.phase` and proceed.

---

## Phase 4: IMPLEMENT

**Goal:** Write code following the plan, milestone by milestone.

**You MUST do all of the following:**

1. **Follow the plan** — Work through milestones in order. Check off tasks as you complete them.

2. **Re-read specs before each milestone** — Don't drift. Load the relevant product and tech specs before starting each milestone to ensure your implementation matches.

3. **Use subagents when helpful:**
   - Spawn background agents for running tests while you continue coding
   - Use Explore agents to find patterns when implementing unfamiliar code paths
   - Use general-purpose agents for complex, independent subtasks

4. **Update progress** — Mark tasks complete in both TodoWrite and `.spec/plan.md` as you go.

5. **Stop and ask if blocked** — If something doesn't match the spec, or the spec is ambiguous, ask the user. Don't guess.

**Rules during this phase:**
- Full tool access — you can write any file
- But every change should trace back to a spec or plan item
- If you discover the plan needs changing, update the plan first, then implement

**Exit criteria:** All planned tasks completed.

**Then:** Write `REVIEW` to `.spec/.phase` and proceed.

---

## Phase 5: REVIEW

**Goal:** Validate the implementation is correct, clean, and complete.

**You MUST do all of the following:**

1. **Run tests** — Execute the project's test suite. Fix any failures.

2. **Run /simplify** — Use the simplify skill for multi-agent review (reuse, quality, efficiency). Apply its suggestions.

3. **Self-review checklist:**
   - [ ] All plan tasks are checked off
   - [ ] Specs are still accurate (update if implementation diverged)
   - [ ] No debug code, console.logs, or TODOs left behind
   - [ ] Code follows existing patterns in the codebase
   - [ ] Lessons learned? Update `.spec/lessons.md`

4. **Validate specs** — Run `bash ~/.agents/skills/spec/scripts/validate.sh`

5. **Present to user** — Show summary of what was built, tests passing, review findings.

**Rules during this phase:**
- Fixes only — no new features
- Every fix traces back to a review finding

**Exit criteria:** Tests pass, review clean, user confirms done.

**Then:** Write `DONE` to `.spec/.phase`.

---

## Phase Transitions Summary

```
RESEARCH → SPEC → PLAN → IMPLEMENT → REVIEW → DONE
    ↑                                    |
    └────────────────────────────────────┘
         (if review reveals spec gaps)
```

## Resuming Work

When `/develop` is called and `.spec/.phase` already exists:
1. Read the current phase
2. Read the relevant specs and plan
3. Present a brief status: "Resuming from IMPLEMENT phase. Last completed: M2. Next: M3."
4. Continue from where you left off

## Context Management

Each phase transition is a natural compaction point. After completing a phase:
1. Ensure all findings/decisions are persisted to files (`.spec/research.md`, specs, plan)
2. The files become the context for the next phase — not your memory of the previous one
3. If context is getting long, suggest `/compact` to the user before starting the next phase

This follows the **Frequent Intentional Compaction** pattern: treat every session as disposable, treat files as permanent. A 10-line spec is worth more than 10,000 tokens of conversation history.

## Subagent Strategy

Use the right subagent for the right job:

| Task | Subagent Type | Model | Why |
|------|--------------|-------|-----|
| Codebase search | `Explore` | haiku | Fast, read-only, cheap |
| Deep code analysis | `Explore` (very thorough) | haiku | Thorough but still cheap |
| Independent implementation task | `general-purpose` | inherited | Needs write access |
| Background test runs | `general-purpose` (background) | inherited | Non-blocking |
| Multi-perspective review | `/simplify` | inherited | 3 parallel review agents |

**Rules for subagents:**
- Always dispatch 3+ independent research tasks in parallel, never sequentially
- Subagents should return **compact summaries**, not raw data (~95% context reduction)
- For implementation, prefer isolated worktree agents for independent modules
- Never let a subagent make architectural decisions — that's the main agent's job

## Key Principle: When In Doubt, Ask

This framework is about doing things right, not doing things fast. If the user's request is ambiguous, ask. If the spec doesn't cover a case, ask. If the plan seems wrong after exploring code, go back and fix the plan. Quality over speed, always.
