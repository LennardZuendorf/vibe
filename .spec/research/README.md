# Research Index

Read this first. This directory contains research that informs the framework design. Each doc answers a specific question and ends with adopt/build recommendations.

Updated: 2026-03-18

---

## Reading Order

For designing the framework, read in this order:

1. **[platform-claude-code.md](platform-claude-code.md)** — What building blocks does Claude Code give us? Skills, hooks, subagents, permissions, settings. Read this to understand what's possible.

2. **[agreements.md](agreements.md)** — Core design decisions already made. Config-driven routing, spec backbone, built-in defaults, cherry-picked patterns. The constraints we're designing within.

3. **[deployment-findings.md](deployment-findings.md)** — Lessons from deploying on a real rework project. New file types (context.md, docs/, reference/), design scope exception, greenfield vs rework gaps.

4. Phase-specific research (read the ones relevant to what you're building):

| Doc | Question | Key Takeaway |
|-----|----------|--------------|
| [phase-research.md](phase-research.md) | How do frameworks do codebase exploration? | Parallel typed agents with fresh contexts, 500-token summaries, disk-based output |
| [phase-discuss.md](phase-discuss.md) | How do frameworks handle scope clarification? | Reconnaissance-first questions, diverge→converge, locked/flexible/deferred decisions |
| [phase-spec.md](phase-spec.md) | How do frameworks handle design specs? | Product/tech separation, mandatory validation, research-informed writing order |
| [phase-plan.md](phase-plan.md) | How do frameworks create implementation plans? | Wave-based grouping, self-contained tasks, plan immutability with revision conditions |
| [phase-implement.md](phase-implement.md) | How do frameworks execute implementation? | Fresh context per task, worktree isolation, git commits as checkpoints, stop-and-ask for ambiguity |
| [phase-review.md](phase-review.md) | How do frameworks validate implementation? | Goal-backward verification, multi-agent review, fixes-only constraint, review artifacts |
| [phase-learn.md](phase-learn.md) | Should learning be a distinct phase? | Yes (Compound Engineering proves it). Structured lessons with confidence scoring, mandatory pruning |

---

## Cross-Cutting Patterns

These patterns appear across multiple phase research docs. They're the "connective tissue" of the framework.

### Pattern: Fresh Context Per Task
**Appears in:** phase-research, phase-implement, phase-review
**Insight:** Subagents should get clean context windows with only what they need. Context rot (>40% utilization) degrades output quality measurably.
**Implication:** Every subagent dispatch includes: task description + relevant spec content + file paths. Never "continue from where we left off."

### Pattern: Disk-Based Communication
**Appears in:** phase-research, phase-spec, phase-plan, phase-implement, phase-learn
**Insight:** Conversation history is ephemeral. Files are permanent. All decisions, findings, and artifacts must be written to `.spec/` to survive compaction and session boundaries.
**Implication:** Every phase produces file artifacts. The next phase reads files, not conversation history.

### Pattern: Parallel Dispatch with Typed Concerns
**Appears in:** phase-research, phase-implement, phase-review
**Insight:** Dispatch 3-4 agents simultaneously, each with a non-overlapping concern (architecture, testing, code quality, dependencies). Prevents duplicate work and maximizes coverage.
**Implication:** Built-in defaults for RESEARCH, IMPLEMENT, and REVIEW all use parallel subagent dispatch.

### Pattern: Phase Gates via Hooks
**Appears in:** platform-claude-code, phase-spec, phase-implement
**Insight:** PreToolUse hooks with exit code 2 physically prevent writes during wrong phases. This is enforcement, not suggestion.
**Implication:** `check-phase.sh` reads `.spec/.phase` and blocks writes that don't match the current phase's rules.

### Pattern: Verify Before Execute
**Appears in:** phase-implement, phase-plan
**Insight:** Re-read the spec before each milestone/wave. Don't drift. If spec is ambiguous, stop and ask — don't guess.
**Implication:** VERIFY phase before IMPLEMENT. Spec re-read before each wave within IMPLEMENT.

### Pattern: Learning as a Distinct Phase
**Appears in:** phase-learn, phase-review, deployment-findings
**Insight:** Review asks "is this correct?" Learning asks "what should we never forget?" Compound Engineering's explicit compound step is their most impactful innovation.
**Implication:** LEARN is a separate phase after REVIEW. Updates lessons.md, prunes stale entries, merges feature specs into global.

---

## What Each Doc Recommends We Build

Consolidated from all research docs. Maps to plan.md tasks.

### From platform-claude-code.md
- Phase gate hook using PreToolUse (exit code 2 to block)
- SessionStart hook for phase state + lessons reminder
- Subagent dispatch patterns (Explore for research, general-purpose for implementation)
- AskUserQuestion for DISCUSS phase
- File-based state (`.spec/.phase`, `.framework.json`)

### From phase-research.md
- 3-4 parallel Explore agents with typed concerns
- research.md artifact with Exists/Must-build/Patterns/Risks/Open-questions structure
- 500-token summary cap per agent
- Two-pass pattern: agents find files → orchestrator reads and understands

### From phase-discuss.md
- Reconnaissance-first questioning (scan codebase before asking)
- Locked/flexible/deferred decision classification
- Scope boundary documentation
- Diverge→converge flow (explore options → commit to one)

### From phase-spec.md
- Product→tech→branches→plan writing order
- Mandatory validation (validate.sh)
- "Alternatives Considered" section in specs
- Completeness gate before moving to PLAN

### From phase-plan.md
- Wave-based task grouping by dependency
- Self-contained tasks (each is dispatchable to a fresh agent)
- Plan immutability with explicit revision conditions
- Verification criteria per task

### From phase-implement.md
- Fresh subagent per task in isolated worktrees
- Background test runs between waves
- Git commits as rollback points
- Spec re-read before each milestone
- Stop-and-ask for ambiguity (never guess)

### From phase-review.md
- Goal-backward verification (spec→code, not code→spec)
- Multi-agent review via /simplify
- Fixes-only constraint (no new features in review)
- Review artifact with verification checklist

### From phase-learn.md
- Structured lessons: Pattern/Rule/Category/Confidence/References/Date
- Automated pruning (check referenced files still exist)
- Feature spec → global spec merge
- 10% time budget (Compound Engineering allocation)

### From deployment-findings.md
- context.md for business/domain context (rework projects)
- docs/ for reference material (API maps, data dictionaries)
- reference/ for visual assets (screenshots, mockups)
- design scope docs crossing product/tech line
- Current State section in product.md for rework projects
- Goal-driven plan phases for rework projects
