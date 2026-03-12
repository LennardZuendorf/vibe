# Agents.md — Framework Vision and Goals

## Project Goal

Build a simple yet powerful spec-driven development framework that:

1. **Aggressively uses subagents** for parallel research, exploration, and review
2. **Front-loads planning** — forces research, discussion, and spec writing before any code
3. **Enforces context adherence** — hooks check that agents stay within their current phase
4. **Follows senior engineer patterns** — research, explore, plan, implement, review
5. **Composes existing skills** — builds on `/spec`, `/simplify`, and Claude's native capabilities

## Design Principles

### 1. Simple Over Complex
The framework is a single `/develop` skill with clear phases, not a sprawling system of dozens of tools. Each phase has a clear entry condition, exit condition, and set of allowed actions.

### 2. Aggressive Subagent Usage
Every research task spawns parallel Explore agents. Every review spawns the simplify multi-agent review. The main agent orchestrates — subagents do the deep work.

### 3. Spec-First, Always
No implementation code exists without a spec that describes it. The spec system (product → tech → plan) is the backbone. Hooks physically prevent file writes during spec phases.

### 4. Phase Gates Are Real
Not guidelines — enforced boundaries. A hook script checks the current phase state file and blocks disallowed tool use. You cannot `Edit` a `.ts` file during the RESEARCH phase.

### 5. Lessons Compound
Every session starts by reading `lessons.md`. Every mistake updates it. Knowledge accumulates across sessions, not just within them.

## Inspiration

- **GSD (Getting Shit Done)** — Phase-gated development with clear boundaries between thinking and doing
- **Ruflo** — Structured AI agent workflows with context persistence
- **Feature-Dev Pattern** — Research → spec → plan → implement → review lifecycle
- **Simplify Skill** — Multi-agent parallel review pattern (reuse, quality, efficiency)

## Architecture Overview

```
.agents/skills/
├── spec/                    # Spec management skill (existing)
│   ├── SKILL.md
│   ├── reference/           # Writing guides + templates
│   └── scripts/             # setup, validate, list
└── develop/                 # Development lifecycle skill (new)
    ├── SKILL.md             # Main skill definition
    └── scripts/
        └── phase-gate.sh    # Phase enforcement script

.claude/
├── settings.json            # Hooks configuration
└── hooks/
    └── check-phase.sh       # PreToolUse hook for phase enforcement
```

## Phase Definitions

### Phase 1: RESEARCH
**Goal:** Understand the problem space and existing codebase
**Allowed:** Read, Glob, Grep, Agent(Explore), WebSearch, WebFetch
**Blocked:** Edit, Write, NotebookEdit, Bash(write operations)
**Exit:** Agent has documented understanding of relevant code areas

### Phase 2: SPEC
**Goal:** Write product and tech specs for the feature
**Allowed:** Read, Glob, Grep, Edit(.spec/), Write(.spec/), Bash(validate.sh)
**Blocked:** Edit(non-spec files), Write(non-spec files)
**Exit:** Specs validated, user approval obtained

### Phase 3: PLAN
**Goal:** Create implementation plan with milestones and tasks
**Allowed:** Read, Glob, Grep, Edit(.spec/plan*), Write(.spec/plan*)
**Blocked:** Edit(non-spec files), Write(non-spec files)
**Exit:** Plan created, user approval obtained

### Phase 4: IMPLEMENT
**Goal:** Write code following the plan
**Allowed:** All tools
**Blocked:** Nothing (full access)
**Exit:** All plan tasks completed

### Phase 5: REVIEW
**Goal:** Validate implementation quality
**Allowed:** Read, Glob, Grep, Edit, Bash(tests/linters), Agent(review)
**Blocked:** Write(new files) — fixes only
**Exit:** Tests pass, /simplify review clean, specs updated if needed

## Current Status

- [x] Spec skill exists and is functional
- [ ] `/develop` skill — needs creation
- [ ] Phase gate hooks — needs creation
- [ ] CLAUDE.md — needs creation
- [ ] Integration testing
