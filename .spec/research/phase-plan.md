# Research: PLAN Phase

> How do AI coding frameworks create structured implementation plans with tasks, dependencies, and validation criteria?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**How it works:**
- Plans are written in **structured XML format** with clear sections
- Tasks are grouped into **waves** — sets of independent tasks that can run in parallel
- Waves execute sequentially (Wave 2 waits for Wave 1)
- Plans are **immutable once approved** — can't change mid-execution
- If reality diverges, a **gap closure protocol** creates a new follow-up plan

**Plan structure (XML):**
```xml
<plan>
  <metadata>
    <title>Feature: Dark Mode Toggle</title>
    <created>2026-03-13</created>
  </metadata>
  <validation>
    <criteria>
      <criterion id="acc-1">Dark mode toggle appears in settings</criterion>
      <criterion id="acc-2">User preference persists across sessions</criterion>
    </criteria>
  </validation>
  <milestones>
    <milestone id="m1" title="Setup State Management">
      <waves>
        <wave id="w1">
          <task id="t1.1">Create theme atom</task>
          <task id="t1.2">Create useTheme hook</task>
          <task id="t1.3">Add persistence logic</task>
        </wave>
      </waves>
    </milestone>
  </milestones>
  <dependencies>
    <dependency from="m2" to="m1">UI depends on state</dependency>
  </dependencies>
</plan>
```

**Key patterns:**
- **Wave-based grouping:** Tasks within a wave have NO dependencies on each other
- **Plan immutability:** Once approved, the plan doesn't change during execution
- **Gap closure protocol:** When reality diverges — document the gap → propose new plan → wait for approval
- **Validation criteria:** What must be TRUE for success (testable, observable)
- **Aggressive atomicity:** Each task is small enough to fit in ~50% of a context window
- **Estimation:** Wave time = slowest task in wave (parallel); Plan time = sum of wave times

**Adopt built-in:** Wave grouping, immutability, gap closure, validation criteria, atomicity
**Skip:** XML format — markdown is more natural for Claude and humans to read/write

---

### Feature-Dev

**How it works:**
- Phase 4 spawns **2-3 `code-architect` agents in parallel**, each with a different mandate
- Typical mandates: minimal-change, clean architecture, pragmatic balance
- Each agent produces a complete architecture blueprint
- Claude reviews all proposals, forms a recommendation, presents trade-offs
- User picks the best approach

**Architecture blueprint includes:**
- Patterns & conventions found in codebase
- Architecture decision (chosen approach + rationale + trade-offs)
- Component design
- Implementation map (files to create/modify)
- Data flow
- Build sequence
- Critical details

**Key patterns:**
- **Competing proposals:** Multiple agents propose different approaches — diversity of thought
- **Mandate-based diversity:** Each agent has a different optimization goal (minimal, clean, pragmatic)
- **Maker-checker loop:** One agent proposes, another evaluates
- **Decisive choices:** Each architect commits to one approach (no wishy-washy "you could do X or Y")
- **Human selection:** User picks from competing proposals — AI diverges, human converges

**Adopt as plugin:** Competing architect agents for complex features
**Built-in equivalent:** Single plan with wave grouping (sufficient for most features)

---

### Superpowers

**How it works:**
- `/superpowers:write-plan` for migrations or multi-file refactors
- `/superpowers:execute-plan` to run plans in batches
- Plans written for a **"dumb executor"** — extremely explicit, no ambiguity
- Each task must be self-contained and understandable without conversation history

**Key patterns:**
- **Explicitness principle:** Plan tasks are written as if for an agent with zero context
- **Self-contained tasks:** Each task includes all the information needed to execute it
- **Batch execution:** Plans run in sequential batches
- **No implicit knowledge:** If a task needs to know about a file, the file path is in the task

**Adopt built-in:** Explicitness principle — every task should be self-contained
**Skip:** Batch execution is less flexible than wave-based grouping

---

### Devin / SWE-Agent / OpenHands

**How they work:**
- **Devin:** Creates a step-by-step plan visible to the user, updates as it works. Flat task list, sequential execution. Can re-plan when stuck.
- **SWE-Agent:** Uses a "thought-action-observation" loop. Plans are implicit — the agent reasons about what to do next at each step.
- **OpenHands:** Similar to SWE-Agent but with a more structured planning phase. Can create explicit plans before executing.

**Key patterns:**
- **Visible planning:** User can see and modify the plan (Devin)
- **Adaptive planning:** Plans update as the agent learns more (Devin, OpenHands)
- **Implicit vs explicit:** SWE-Agent plans implicitly (just acts); Devin plans explicitly (shows steps)

**Adopt pattern:** Plan visibility (we already do this — plan.md is readable)
**Skip:** Adaptive planning conflicts with immutability — we prefer gap closure instead

---

### Aider

**How it works:**
- No formal planning phase — Aider goes straight from prompt to implementation
- For multi-file changes, it identifies affected files and edits them
- Uses its repo map to understand what needs to change
- Plans are implicit in the way it sequences edits

**Skip:** No structured planning patterns to adopt

---

## Synthesis: Recommendations for Our PLAN Phase

### Built-in Default Provider

```
1. Read specs:
   - Load product and tech specs for the feature
   - Load research findings for context

2. Create wave-grouped plan in .spec/plan.md:

   ## Milestone 1: [Name]
   **Goal:** [What this achieves]
   **Validation:** [What must be TRUE when done]

   ### Wave 1 (parallel)
   - [ ] **Task 1.1:** [Self-contained description with file paths]
   - [ ] **Task 1.2:** [Self-contained description with file paths]

   ### Wave 2 (depends on Wave 1)
   - [ ] **Task 2.1:** [Description referencing Wave 1 outputs]

3. Set validation criteria:
   - Each milestone has testable success conditions
   - Final validation covers the full feature

4. Present to user: "Ready to implement? Changes?"
```

### Plan Format (Markdown, Not XML)

We use markdown for plans because:
- Claude reads/writes markdown natively
- Humans can read/review easily in any editor
- Checkboxes (`- [ ]`) provide built-in progress tracking
- Compatible with our spec system
- No parsing infrastructure needed

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **feature-dev** | Spawn 2-3 competing architect agents with different mandates. Present proposals to user. Especially useful for complex features where the approach isn't obvious. |
| **superpowers** | Enforce "dumb executor" explicitness — each task must be self-contained with all context included |

### Key Design Decisions

1. **Wave-based grouping (from GSD).** Tasks within a wave have no dependencies. Waves run sequentially. This enables parallel subagent execution during IMPLEMENT.
2. **Plan immutability (from GSD).** Once approved, the plan doesn't change. If reality diverges, use gap closure: document gap → propose new plan → get approval.
3. **Validation criteria per milestone.** Not just "did we do the tasks?" but "what must be TRUE?" (goal-backward verification).
4. **Self-contained tasks (from Superpowers).** Each task includes all context needed for a fresh agent to execute it. No implicit knowledge.
5. **Markdown format.** Not XML. Checkboxes for tracking. Compatible with our spec system.
6. **Atomic tasks.** Each task should fit in ~50% of a context window. If it's bigger, break it down.

---

## Sources

- [GSD Framework (GitHub)](https://github.com/gsd-build/get-shit-done)
- [GSD Wave Execution](https://github.com/gsd-works/framework/blob/main/docs/waves.md)
- [GSD Plan Immutability](https://github.com/gsd-works/framework/blob/main/docs/immutability.md)
- [Feature-Dev code-architect](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-architect.md)
- [AI Agent Orchestration Patterns (Microsoft)](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
