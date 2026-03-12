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

## Inspiration & References

- **GSD (Getting Shit Done)** — Phase-gated development with clear boundaries between thinking and doing. Each phase runs in a fresh context window; requirements trace to phases, phases to plans, plans to commits. (ccforeveryone.com/gsd)
- **HumanLayer FIC (Frequent Intentional Compaction)** — Validated on 300K+ LOC Rust codebases. Core insight: subagents write findings to files, not inline. Main context stays clean. Enables 35K LOC feature additions in 7 hours. (github.com/humanlayer/advanced-context-engineering-for-coding-agents)
- **cc-sdd** — Kiro-style commands enforcing Requirements → Design → Tasks → Implementation. Multi-tool compatible. (github.com/gotalab/cc-sdd)
- **claude-wizard** — 8-phase workflow with TDD, adversarial review, and quality gate cycles. (github.com/vlad-ko/claude-wizard)
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

## Key Patterns

### Research-to-File
Subagents write findings to `.spec/research.md`, not inline. This preserves main context (~95% reduction) and creates a durable artifact other phases can reference.

### Frequent Intentional Compaction
Each phase transition is a compaction point. Files are the memory, not conversation history. A 10-line spec outweighs 10,000 tokens of chat.

### Lock-File Phase Gates
Hooks check `.spec/.phase` before allowing writes. Exit code 2 = hard block. This is deterministic — not a suggestion the model can ignore.

### Stop-Hook Quality Gates
End-of-turn hooks can run tests and linters automatically after every response, catching regressions immediately rather than at review time.

## Current Status

- [x] Spec skill — functional
- [x] `/develop` skill — created with 5-phase lifecycle
- [x] Phase gate hooks — PreToolUse enforcement via check-phase.sh
- [x] SessionStart hook — context injection at session start
- [x] CLAUDE.md — framework overview and rules
- [x] Agents.md — vision, goals, architecture

### Future Enhancements
- [ ] Stop hook for end-of-turn quality gates (auto-test, auto-lint)
- [ ] PostToolUse hook for auto-formatting after edits
- [ ] Custom `.claude/agents/` subagent definitions with persistent memory
- [ ] `.mcp.json` for GitHub/external service integration
- [ ] `.claude/rules/` for path-specific coding standards
- [ ] Bash command filtering in phase-gate (block `rm`, `mv` during RESEARCH)
