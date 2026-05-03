# Develop Skill — Spec-Driven Feature Lifecycle

> **For humans:** This README explains how the develop skill works. Claude reads SKILL.md instead.

## What This Skill Does

The **develop** skill orchestrates a complete feature development lifecycle through 5 mandatory phases. It enforces what senior engineers do naturally: research before planning, plan before coding, review before shipping.

## Quick Start

```bash
# Start developing a feature
/develop add user authentication with OAuth

# Resume an in-progress feature
/develop                    # Reads .spec/.phase and resumes
```

## The 5 Phases

```
RESEARCH → SPEC → PLAN → IMPLEMENT → REVIEW → DONE
```

### Phase 1: RESEARCH
- Spawns parallel Explore subagents to understand codebase
- Reads existing specs and lessons
- Searches for related code, tests, types
- Presents findings for user confirmation
- **No file writes allowed**

### Phase 2: SPEC
- Writes product specs (WHAT & WHY) via `/spec`
- Writes tech specs (HOW) via `/spec`
- Validates spec consistency
- Presents specs for user approval
- **Only `.spec/` writes allowed**

### Phase 3: PLAN
- Creates implementation plan with milestones
- Estimates sessions, identifies risks
- Sets up task tracking
- Presents plan for user approval
- **Only `.spec/plan*.md` writes allowed**

### Phase 4: IMPLEMENT
- Follows the plan milestone by milestone
- Re-reads specs before each milestone
- Uses subagents for parallel work
- Tracks progress in real-time
- **Full tool access**

### Phase 5: REVIEW
- Runs test suite
- Runs `/simplify` for multi-agent review
- Self-review checklist
- Updates lessons learned
- **Fixes only, no new features**

## Phase Enforcement

Phase gates are enforced by hooks, not guidelines. The `check-phase.sh` hook runs before every Edit/Write/NotebookEdit and blocks writes that violate the current phase.

State is stored in `.spec/.phase` as a simple text file.

## Subagent Patterns

The develop skill uses subagents aggressively:

| Phase | Subagent Usage |
|-------|---------------|
| RESEARCH | 3+ parallel Explore agents for codebase search |
| SPEC | Agent for exploring unfamiliar areas while writing |
| PLAN | Explore agents for validation of estimates |
| IMPLEMENT | Background agents for tests, Explore for patterns |
| REVIEW | `/simplify` spawns 3 review agents (reuse, quality, efficiency) |

## Integration with /spec

The develop skill delegates all spec management to the `/spec` skill. It doesn't duplicate spec logic — it orchestrates when and how specs are created and used.

## Files

- `SKILL.md` — Skill definition (Claude reads this)
- `README.md` — Human documentation (you're reading this)
- `scripts/phase-gate.sh` — Phase enforcement logic
