# Research: IMPLEMENT Phase

> How do AI coding frameworks handle code execution, subagent orchestration, and multi-file changes?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**Core architecture:**
- **Context budget model:** Orchestrator uses ~15% of its context. Each executor gets 100% fresh context. Quality is measurable: 0-30% context = peak quality; 50%+ = corner-cutting; 70%+ = hallucinations.
- **Plans as prompts:** `PLAN_N_M.md` files ARE the executable instructions read directly by subagents. Each plan is capped at 3 tasks. Atomicity enforced structurally, not by convention.
- **Tasks sized to fit ~50% of a fresh context window.** This ensures quality never degrades.

**Task dispatch and parallelism:**
- Thin orchestrator pattern: main session never does heavy lifting
- Analyzes plan dependencies from frontmatter, groups independent plans into waves
- Spawns `gsd-executor` agents in parallel. Wave 1 completes before Wave 2 begins
- Plans grouped as **vertical slices** (end-to-end features) rather than horizontal layers — minimizes inter-plan dependencies

**Context management:**
- Every task produces a git commit — serves as rollback point AND checkpoint reconstruction
- After compaction, agent reads `git log` and `git diff` to reconstruct state (never relies on conversation memory)
- Main session stays at 30-40% utilization

**Gap closure:**
- After verification (`/gsd:verify-work`), gaps identified
- `/gsd:plan-milestone-gaps` creates targeted gap-closure phases with `gap_closure: true` in frontmatter
- `--gaps-only` flag executes only these plans
- Dedicated `/gsd:debug` spawns a debugger agent with `goal=find_and_fix`

**GSD v2 additions (CLI-based):**
- Different models for different tasks (Opus for planning, Sonnet for execution)
- Direct context management — clear context between tasks
- Inject exactly the right files at dispatch time
- Git branch management per task
- Cost and token tracking
- Stuck loop detection and crash recovery

**Key insights:**
- **Context budget is the core innovation.** Everything else follows from keeping the orchestrator lean.
- **Git commits as checkpoints** — after compaction, `git log` is the state, not conversation history.
- **Vertical slices over horizontal layers** — minimizes cross-task dependencies.

---

### Superpowers

**TDD enforcement (punitive):**
- RED-GREEN-REFACTOR enforced structurally
- Agent writes failing test → confirms it fails → writes minimal code to pass
- **If code is written before a test, the framework DELETES it** and forces restart from test-first position
- Philosophical basis: tests-first answer "what should this do?" while tests-after answer "what does this do?" — tests after are biased by the implementation

**Dual-pass review (per task):**
- After each task subagent completes, two additional subagents dispatched sequentially:
  1. **Spec compliance reviewer** — confirms code matches specification
  2. **Code quality reviewer** — approves or requests fixes
- Critical issues block progression — orchestrator doesn't advance until both pass

**Subagent dispatch sequence:**
```
1. Dispatch implementer subagent (fresh context) with task + spec
2. Implementer asks clarifying questions if needed
3. Implementer implements, tests, commits, self-reviews
4. Dispatch spec reviewer subagent
5. Dispatch code quality reviewer subagent
6. Mark task complete OR route back to implementer with specific fixes
```

**Sequential vs. parallel:**
- Current pattern is sequential subagent dispatch (one at a time)
- Pending proposal (Issue #469) introduces team-based parallel execution using `TeamCreate` and `SendMessage` APIs with shared `TaskList`
- Human review checkpoints remain between batches

**Key insights:**
- **Punitive TDD is the differentiator.** No other framework enforces test-first this aggressively.
- **Three subagents per task** (implementer + 2 reviewers) is expensive but catches issues early.
- **Sequential dispatch is a known limitation** — parallel execution is planned but not shipped.

---

### Feature-Dev / Agentic Development Process

**Checkpoint-based execution:**
- 10 mandatory quality gates before any merge
- Each checkpoint is a validation condition, not a time-based pause
- File-based communication: master writes task specs to files, subagents read/execute, results written back

**Worktree isolation:**
- Master assigns each subagent to its own git worktree
- Two agents can safely edit the same file with different approaches
- Results compared via standard `git diff`, winning implementation chosen or merged

---

### Claude Code (Our Execution Engine)

**Agent tool capabilities:**
- Spawn subagents: `general-purpose`, `Explore`, `Plan` types
- `isolation: "worktree"` — temporary worktree per subagent, auto-cleaned if no changes
- `run_in_background: true` — non-blocking execution
- Multiple Agent calls in a single message = parallel dispatch
- `model: "sonnet"` or `model: "haiku"` per subagent
- `resume: "agentId"` to continue previous agent's work

**Performance data:**
- Anthropic measured **90.2% better performance** from Opus orchestrator + Sonnet subagents vs. Opus alone
- 15× token cost increase for multi-agent vs. single-agent (primary tradeoff)
- Standard pattern: Opus for orchestrator (complex reasoning), Sonnet for executors (focused tasks)

**Worktree constraints:**
- Without worktrees: parallel agents limited to non-overlapping file paths (fragile)
- With worktrees: each agent has entire codebase to itself
- Known limitations: port conflicts, shared database state
- Workaround: `BASE_PORT + (WORKTREE_INDEX * 10) + SERVICE_OFFSET`
- Disk amplification: ~5× for 2GB codebase with 5 worktrees

---

### Devin / SWE-Agent / OpenHands

**Devin:**
- Agentic loop: decompose → search docs → edit → run tests → analyze failures → iterate
- Persistent memory across sessions
- Devin 2.0: 83% more tasks per compute unit than v1
- Real-world success rate ~15% on complex tasks (matches SWE-bench 13.86%)
- **Key failure mode:** Optimizing for wrong goal when spec is ambiguous (e.g., IP analysis vs. user-level analysis)

**OpenHands:**
- **Event-sourced architecture:** All interactions are immutable events appended to a log
- Enables replay, fault recovery, deterministic state reconstruction
- `LLMSummarizingCondenser`: replaces old conversation history with summaries (~2× cost reduction)
- Docker-based sandboxing per agent/session
- HIGH uncertainty actions held for human review (configurable thresholds)
- SWE-bench Verified: **72% resolution rate** (highest in survey) using Claude Sonnet 4.5 + extended thinking

**SWE-Agent:**
- Think-act loop: plan → act → observe → verify → continue/revise
- If tool call fails, detects failure and tries alternative strategy
- Adaptive replanning at the step level

---

### Aider

**Multi-file editing:**
- Repository map gives structural context (function signatures, file structure) without loading all contents
- Only files explicitly added via `/add` are in edit context — deliberate constraint prevents scope creep
- Edit format is model-specific: `editblock` (diff format) for capable models

**Git integration:**
- Every edit produces a commit with semantically generated message
- Commits marked with `(aider)` in author metadata
- Pending changes committed before Aider edits (clean rollback boundary)
- **Architect mode:** Discusses structure/design before making changes

---

## Cross-Cutting Patterns

### Subagent Dispatch Invariant

All frameworks converge on: **one focused objective per subagent, one concrete deliverable, fresh context.**

Context hierarchy:
```
Orchestrator (~15% context)
  → Executor subagents (100% fresh, one task each)
    → Reviewer subagents (100% fresh, spec then quality)
```

Subagent outputs synthesized before orchestration decisions — never passed raw.

### Context Management (4 Mechanisms)

| Mechanism | Used By | How It Works |
|-----------|---------|-------------|
| **Atomic task sizing** | GSD (3 tasks max), Superpowers (2-5 min tasks) | Tasks fit in ~50% of fresh context |
| **Git commits as checkpoints** | GSD, Aider | After compaction, `git log` + `git diff` reconstruct state |
| **Progress files on disk** | GSD `STATE.md`, OpenHands event log | Structured summaries written at each transition |
| **Context compaction** | OpenHands `LLMSummarizingCondenser`, Claude `/compact` | Replace history with structured summaries |

### Test Integration (3 Patterns)

| Pattern | Framework | Description |
|---------|-----------|-------------|
| **Blocking TDD** | Superpowers | Tests gate code. No code before failing test. Delete code if written first. |
| **Background test runs** | Claude Code, GSD | After wave, background agent runs tests while orchestrator plans next wave. |
| **Commit-on-green** | Aider, GSD | Each task committed only when tests pass. Git log = verified history. |

### Error Recovery Hierarchy

| Level | Mechanism | Frameworks |
|-------|-----------|------------|
| **Step** | Think-act loop, alternative tool strategy | SWE-Agent, Devin, OpenHands |
| **Task** | Git revert to pre-task commit, replan locally | GSD, Aider |
| **Review** | Spec reviewer blocks, routes back to implementer | Superpowers |
| **Phase** | Gap closure plans, targeted re-execution | GSD |
| **Session** | Event log replay, checkpoint rewind | OpenHands, Claude Code |

### Detecting Plan Divergence (3 Levels)

1. **Step-level** (SWE-Agent, Devin): Think-act loop — each tool call result observed before next action
2. **Task-level** (Superpowers): Spec-review subagent reads spec and code, reports whether they match. Catches semantic divergence tests can't catch.
3. **Phase-level** (GSD): Verify-and-gap-close cycle — verification against requirements, failures generate targeted closure plans

---

## Synthesis: Recommendations for Our IMPLEMENT Phase

### Built-in Default Provider

```
For each wave in the plan:

  1. Pre-check:
     - Re-read specs relevant to this wave's tasks
     - Load any outputs from previous waves

  2. Dispatch tasks (parallel within wave):
     - Each subagent gets: task description + relevant spec excerpts + file paths
     - For independent file changes: worktree-isolated agents
     - For simple tasks: standard agents
     - Model: Sonnet for executors (focused tasks)

  3. Collect results:
     - Each agent returns: compact summary, files changed, tests passing (y/n), blockers
     - Merge worktree changes back
     - Git commit per wave (checkpoint)

  4. Verify:
     - Run tests in background
     - Check for gap: does implementation match plan?
     - If gap → gap closure protocol

  5. Update progress:
     - Mark tasks complete in plan.md
     - Update TodoWrite
     - Write structured state to .spec/.state (phase, milestone, last task)

  6. Next wave or done
```

### Error Recovery Hierarchy

```
Step fails → subagent replans locally, tries alternative
  ↓ (2+ attempts fail)
Task fails → surface blocker to orchestrator, halt task
  ↓
Spec review fails → route back to implementer with specific requirements
  ↓
Wave fails → orchestrator reviews gap, updates plan, creates closure tasks
  ↓
Phase fails → return to PLAN phase (don't patch forward from broken state)
```

### The Single Most Important Heuristic

From the cross-framework analysis: **When the spec is ambiguous and implementation requires a judgment call, STOP AND ASK.** Devin's documented failure mode — optimizing for the wrong goal — is representative. The cost of asking is one round-trip. The cost of guessing wrong is a full re-implementation.

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **superpowers** | Punitive TDD: write failing test → confirm failure → implement → pass. Delete code written before tests. Per-task spec compliance reviewer + quality reviewer. |

### Key Design Decisions

1. **Fresh subagent per task (GSD).** 100% clean context. No degradation over long sessions.
2. **Worktree isolation for parallel tasks.** Isolated git worktrees — no file conflicts.
3. **Wave-based execution.** Parallel within wave, sequential across waves.
4. **Context budget: keep orchestrator lean (~15%).** Heavy work done by subagents.
5. **Git commits as checkpoints (GSD, Aider).** After compaction, `git log` reconstructs state.
6. **Gap closure over patching forward (GSD).** Stop → document → re-plan → get approval → continue.
7. **Background test runs.** Tests run while next wave is planned (non-blocking).
8. **Spec re-reading before each milestone.** Prevents drift.
9. **When in doubt, ask.** Never guess at ambiguous specs.
10. **Vertical slices over horizontal layers (GSD).** Minimizes cross-task dependencies.

---

## Sources

- [GSD Framework](https://gsd.build/)
- [GSD context budget management](https://zread.ai/gsd-build/get-shit-done/19-context-budget-management)
- [GSD vs Spec Kit comparison](https://medium.com/@richardhightower/agentic-coding-gsd-vs-spec-kit-vs-openspec-vs-taskmaster-ai-where-sdd-tools-diverge-0414dcb97e46)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [subagent-driven-development SKILL.md](https://github.com/obra/superpowers/blob/main/skills/subagent-driven-development/SKILL.md)
- [executing-plans SKILL.md](https://github.com/obra/superpowers/blob/main/skills/executing-plans/SKILL.md)
- [TDD enforcement: stop AI from writing spaghetti](https://yuv.ai/blog/superpowers)
- [Red/green TDD in agentic patterns](https://simonwillison.net/guides/agentic-engineering-patterns/red-green-tdd/)
- [Claude Code sub-agents docs](https://code.claude.com/docs/en/sub-agents)
- [Claude Code worktrees guide](https://claudefa.st/blog/guide/development/worktree-guide)
- [Parallel AI coding with git worktrees](https://docs.agentinterviews.com/blog/parallel-ai-coding-with-gitworktrees/)
- [Claude Code agent teams guide](https://www.heyuan110.com/posts/ai/2026-02-28-claude-code-teams-guide/)
- [Agentic development process](https://github.com/Klz-1/agentic-development-process)
- [Devin AI autonomous engineering](https://www.digitalapplied.com/blog/devin-ai-autonomous-coding-complete-guide)
- [OpenHands vs Devin](https://modelgate.ai/blogs/ai-automation-insights/openhands-vs-devin-autonomous-ai-software-engineer)
- [OpenHands SDK paper (arXiv)](https://arxiv.org/html/2511.03690v1)
- [Aider git integration](https://aider.chat/docs/git.html)
- [Why multi-agent systems fail](https://towardsdatascience.com/why-your-multi-agent-system-is-failing-escaping-the-17x-error-trap-of-the-bag-of-agents/)
