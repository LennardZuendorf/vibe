# Research: IMPLEMENT Phase

> How do AI coding frameworks handle code execution, subagent orchestration, and multi-file changes?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**How it works:**
- Each task dispatched to a **fresh subagent** with a clean 200k context window
- Wave-based execution: all tasks in Wave N complete before Wave N+1 starts
- Tasks within a wave run **in parallel** via concurrent subagents
- Main session stays at 30-40% context while subagents do heavy lifting
- **Gap closure:** If a task reveals the plan doesn't match reality, execution stops → gap is documented → new plan proposed → human approves → execution continues

**GSD v2 additions (CLI-based):**
- Different models for different tasks (Opus for complex, Sonnet for standard, fast model for boilerplate)
- Direct context management — can clear context between tasks
- Inject exactly the right files at dispatch time
- Git branch management per task
- Cost and token tracking
- Stuck loop detection and crash recovery
- Auto-advance through milestones without human intervention

**Key patterns:**
- **Fresh context per task:** Task 50 has same quality as task 1 (no degradation)
- **Aggressive atomicity:** Each task fits in ~50% of a context window
- **Wave parallelism:** Independent tasks execute concurrently
- **Gap closure protocol:** Stop → document → re-plan → continue
- **File injection:** Subagents get only the files they need, not the entire codebase
- **Stuck detection:** Recognize when an agent is going in circles

**Adopt built-in:** Fresh contexts, wave parallelism, gap closure, atomicity
**Skip:** CLI infrastructure, auto-advance (we want human gates), model routing (open question)

---

### Superpowers

**How it works:**
- **TDD enforcement:** Write tests first (red), then implementation (green), then refactor
- **Dual-pass writing:** First pass writes code, second pass reviews it
- Subagent-driven development with specialized sub-agents operating in parallel
- Extended thinking (`ultrathink`) for complex implementation decisions

**Key patterns:**
- **Red-green-refactor:** Tests must FAIL before writing implementation. Forces you to define "done" before coding.
- **Dual-pass:** Write once, review once. Catches issues before they compound.
- **Parallel sub-agents:** Different agents handle different aspects (infrastructure, UI, tests)
- **Ultrathink:** Extended reasoning for non-trivial implementation decisions

**Adopt as plugin:** TDD enforcement (optional per project)
**Built-in equivalent:** Run tests after each wave (not test-first, but test-verified)

---

### Feature-Dev

**How it works:**
- Phase 5 ("Implementation") starts only after architecture is approved
- Reads all files identified during exploration phases
- Implements following the approved architecture approach
- Respects conventions found during exploration
- Tracks progress throughout

**Key patterns:**
- **Architecture-first:** Implementation follows the approved blueprint exactly
- **Convention respect:** Uses patterns found during research, not inventing new ones
- **File pre-reading:** Reads all relevant files before writing any changes
- **Progress tracking:** Visible checkpoints during implementation

**No unique execution patterns to adopt** — our wave-based approach is more sophisticated

---

### Claude Code (Vanilla Capabilities)

**What's available natively:**
- **Agent tool:** Spawn subagents (general-purpose, Explore, Plan types)
- **Worktree isolation:** `isolation: "worktree"` gives a subagent its own git worktree copy
- **Background execution:** `run_in_background: true` for non-blocking tasks
- **Parallel dispatch:** Multiple Agent calls in a single message
- **Model override:** `model: "sonnet"` or `model: "haiku"` per subagent
- **Resume:** `resume: "agentId"` to continue a previous agent's work

**Key patterns:**
- **Worktree isolation for file independence:** Each agent works on an isolated copy — no merge conflicts during parallel work
- **Background test runs:** Spawn a test-runner agent in background while starting next wave
- **Model selection:** Use cheaper/faster models for simple tasks, expensive models for complex ones
- **Compact summaries:** Agents return summaries, not raw output

**This IS our execution engine.** Everything else describes how to orchestrate it.

---

### Devin / SWE-Agent / OpenHands

**How they work:**
- **SWE-Agent:** Thought-action-observation loop. Acts, observes result, reasons about next step. Single-threaded, sequential.
- **OpenHands:** Similar loop but can plan ahead. Supports runtime sandboxes for testing.
- **Devin:** Multi-step execution with visible plan. Can browse web, run terminals, use editor. Sequential but can revisit steps.

**Key patterns:**
- **Observation-driven:** Check results after each action, adjust if needed
- **Sandbox execution:** Run code in isolated environments to verify
- **Step revisiting:** Can go back and redo earlier steps (Devin)

**Adopt pattern:** Observation after each wave (run tests, check for issues)
**Skip:** Sequential single-threaded execution — too slow for our use case

---

### Aider

**How it works:**
- Uses a "whole file" or "diff" editing mode
- Identifies affected files from its repo map
- Makes changes across multiple files in a single pass
- Automatically creates git commits for each change
- Can undo changes via git if they're wrong

**Key patterns:**
- **Auto-commit:** Each change set gets a git commit (easy rollback)
- **Diff-based edits:** Only sends the changes, not full files
- **Undo via git:** If something breaks, just revert the commit

**Adopt pattern:** Consider auto-committing after each wave for easy rollback
**Skip:** Single-pass editing — we prefer wave-based parallel execution

---

## Synthesis: Recommendations for Our IMPLEMENT Phase

### Built-in Default Provider

```
For each wave in the plan:

  1. Pre-check:
     - Re-read specs relevant to this wave's tasks
     - Load any outputs from previous waves

  2. Dispatch tasks (parallel within wave):
     - For independent file changes: spawn worktree-isolated agents
     - For simple tasks: spawn standard agents
     - Each agent gets: task description + relevant spec excerpts + file paths

  3. Collect results:
     - Merge worktree changes back
     - Review agent summaries for issues

  4. Verify:
     - Run tests in background
     - Check for gap: does implementation match plan?
     - If gap found → gap closure protocol

  5. Update progress:
     - Mark tasks complete in plan.md
     - Update TodoWrite
     - If tests fail → fix before advancing

  6. Next wave or done
```

### Wave Execution Diagram

```
Wave 1:  [Task A] ──┐
         [Task B] ──┼── merge → test → ✓
         [Task C] ──┘

Wave 2:  [Task D] ──┐
         [Task E] ──┼── merge → test → ✓
                    ┘

Wave 3:  [Task F] ──── final test → review
```

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **superpowers** | TDD enforcement: write test first (red) → write implementation (green) → refactor. Each task becomes a red-green-refactor cycle. |

### Key Design Decisions

1. **Fresh subagent per task (from GSD).** Every task gets a clean context. No degradation over long implementation sessions.
2. **Worktree isolation for parallel tasks.** Independent tasks run in isolated git worktrees — no file conflicts.
3. **Wave-based execution.** Parallel within wave, sequential across waves. Matches the plan structure.
4. **Gap closure (from GSD).** If reality diverges from plan: stop → document the gap → update plan → get approval → continue.
5. **Background test runs.** After each wave completes, run tests in background while starting next wave (if independent).
6. **Spec re-reading before each milestone.** Prevents drift — re-load relevant specs before starting each milestone.
7. **No auto-advance.** Human confirms at milestone boundaries (not at every wave, but at milestone gates).

### Error Recovery

```
Task fails → Agent reports failure with context
  → Main agent analyzes: is this a plan gap or a code bug?
    → Plan gap: trigger gap closure protocol
    → Code bug: retry with more context, or ask user
  → If stuck (3+ retries): escalate to user
```

---

## Sources

- [GSD Framework (GitHub)](https://github.com/gsd-build/get-shit-done)
- [GSD v2](https://github.com/gsd-build/gsd-2)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Claude Code Agent tool documentation](https://code.claude.com/docs/en/agents)
- [Codified Context (arXiv)](https://arxiv.org/html/2602.20478v1)
