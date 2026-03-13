# Research: PLAN Phase

> How do AI coding frameworks create structured implementation plans with tasks, dependencies, and validation criteria?

Updated: 2026-03-13

---

## Framework Analysis

### GSD (Get Stuff Done)

**Task structure: XML inside markdown**
- Plans live in `PLAN.md` files that are **executable prompts first, human-readable second**
- Tasks use a rigid XML schema with `<n>`, `<files>`, `<action>`, `<verify>`, `<done>` tags
- `type` attribute: `auto` (runs unattended) or `checkpoint` (pauses for human review)
- `<action>` must include what to avoid and WHY — not optional
- Maximum **3 tasks per plan file**
- Each plan sized to fit ~50% of a fresh 200k-token context window

**Example:**
```xml
<task type="auto">
  <n>Task 1: Create JWT middleware</n>
  <files>src/middleware/auth.ts, src/types/jwt.ts</files>
  <action>
    Add JWT verification middleware. Use jose library (NOT jsonwebtoken —
    CommonJS incompatibility with Edge runtime). Handle expired with 401,
    malformed with 400.
  </action>
  <verify>pnpm test src/middleware/auth.test.ts && curl -H "Authorization: Bearer invalid" localhost:3000/api/profile | grep 401</verify>
  <done>All auth.test.ts tests pass. Manual curl with invalid token returns 401.</done>
</task>
```

**Wave grouping:**
- Tasks grouped into waves based on dependency topology
- If task B depends on task A, they cannot share a wave
- Tasks in same wave are fully independent → parallel subagents
- **Vertical slices over horizontal layers** — vertical slices parallelize better
- Wave time = slowest task in wave (parallel); plan time = sum of wave times

**Plan immutability + gap closure:**
- Once created, a plan file is **never edited**
- When verification finds failures: gap-closure plans created with `gap_closure: true` in frontmatter
- `--gaps-only` flag executes only gap plans, skipping completed work
- "Surgeon, not Architect" rule: make minimal targeted change, never rewrite a working plan

**Dependencies are implicit in wave number.** Wave 3 signals dependence on all Wave 1 and 2 outputs. No explicit `depends_on` syntax.

**Adopt built-in:** Wave grouping, immutability, gap closure, verification commands, atomic task sizing, vertical slices
**Skip:** XML format — markdown is more natural for Claude and humans

---

### Feature-Dev

**How it works:**
- `code-architect` agent produces a **single decisive architecture blueprint**
- Does NOT present multiple options — picks one and commits
- Blueprint sections: Patterns Found, Architecture Decision, Component Design, Implementation Map, Data Flow, Build Sequence, Critical Details
- Build Sequence = ordered checklist (closest to a plan), sequential by position

**Competing proposals (multi-agent extension):**
- Multiple `code-architect` instances analyze the same feature request
- Each produces a complete blueprint independently
- Orchestrator or human selects the best proposal
- Azure's "competitive pattern": yields gains on uncertain/parallelizable tasks
- Google Research finding: **substantial gains on uncertain tasks, can degrade on sequential tasks**

**Adopt as plugin:** Competing architects for complex features with uncertain approach
**Built-in equivalent:** Single plan with wave grouping

---

### Superpowers

**"Plan for a dumb executor" principle:**
- Write instructions as if for "an enthusiastic junior engineer with poor taste, no judgement, no project context, and an aversion to testing"
- Include **exact file paths**, not descriptions
- Include **complete code snippets**, not partial examples
- Include **runnable verification commands**, not abstract criteria
- Pre-write git commit messages

**Task structure:**
- Markdown checklist with embedded code
- Each task: file paths, complete code change, verification command, expected output, pre-written commit message
- Tasks estimated at **2-5 minutes each**. If longer, split.

**Hard phase gates (v4.3.0):**
- Writing-plans skill inserts a block preventing implementation code until plan is presented and approved
- Earlier versions relied on instructions alone — model would rationalize skipping

**Adopt built-in:** Dumb executor explicitness, task sizing, hard phase gates
**Skip:** Embedded complete code (too rigid for most changes)

---

### BMAD Method

**YAML-based workflow blueprints with explicit DAG:**
```yaml
workflow:
  name: feature-user-auth
  steps:
    - id: prd
      agent: pm
      action: write-prd
      depends_on: [brief]
    - id: architecture
      agent: architect
      action: design-architecture
      depends_on: [prd]
    - id: stories
      agent: scrum-master
      action: create-stories
      depends_on: [architecture, prd]
```

**Key difference from GSD:** Uses explicit `depends_on: [id]` arrays — a **proper DAG notation** rather than implicit wave numbering.

**Hyper-detailed stories:**
- Story files embed everything an executor needs: full context, code examples, acceptance criteria, architecture decisions
- BMAD equivalent of "plan for a dumb executor"

**Plan mutability:** Living documents updated in-place. Scrum Master can revise mid-sprint.

**Adopt pattern:** Explicit `depends_on` notation alongside waves for clarity
**Skip:** Full persona system, YAML format

---

### Cursor / Windsurf Cascade

**Windsurf:** Two-tier architecture — background planning agent continuously refines plan while execution tier handles current action. Plan exposed as Todo list. **Plan is mutable during execution** (opposite of GSD).

**Cursor:** Separates planning (Chat/Ask mode, no writes) from execution (Agent/Composer mode, writes enabled). Transition is manual — user-enforced phase gate.

**Skip:** Mutable-during-execution conflicts with our immutability principle
**Adopt pattern:** Cursor's explicit mode switching mirrors our phase gates

---

### Devin 2.0

**Interactive planning as first-class:**
1. User describes task
2. Devin scans repo and **generates a step-by-step plan**
3. User reviews and edits plan
4. User approves → Devin executes in sandbox
5. User can inject mid-flight clarifications; Devin updates plan

**Plan is adaptive** — changes frequently as execution proceeds. Not treated as immutable.

**Confidence signaling:** After execution, Devin provides confidence level for task completion. Triggers human review when low.

**Skip:** Adaptive planning conflicts with our immutability model
**Adopt pattern:** Plan generation from codebase analysis (we do this via specs)

---

### SWE-Agent / OpenHands

**OpenHands:** Primarily ReAct loop (implicit planning). Explicit planning mode in development:
- **Plan mode (read-only):** Agent maintains `PLAN.md` with success criteria, implementation steps, context notes
- **Execute mode (write-enabled):** Plan handed to executor agent

**Agent trajectory analysis:** ~50% exploration in first 20% of task, shifting to code/test/debug in final 20%. Average: 29 iterations.

**Adopt pattern:** The plan-then-execute separation (we already have this)

---

### Aider

**Architect/Editor split:**
- **Architect model:** Full reasoning, produces natural-language description of all changes
- **Editor model:** Converts to precise file-edit instructions
- Strong reasoning model + cheap editor = better than either alone
- **No persistent plan artifact** — plan exists only in-context (weakest aspect)

**Adopt pattern:** Separate reasoning from task writing
**Skip:** Ephemeral plans — we need persistence

---

### GAP: Graph-Based Agent Planning (NeurIPS 2025)

**Key academic contribution:**
- Formalizes planning as a **directed acyclic graph**: G = (V, E) where V = subtasks, E = dependencies
- Waves derived algorithmically via topological sort: wave N = all tasks whose longest dependency chain is N-1
- **Finding:** Traditional ReAct agents fail to exploit parallelism. GAP achieves substantial accuracy AND efficiency improvements.
- **Contract-based inter-task dependencies:** Producer-consumer contracts define exact output format between tasks

**Adopt pattern:** Algorithmic wave derivation from dependency edges (formalization of what GSD does intuitively)

---

### LangChain Plan-and-Execute

**Three-node architecture:** `Planner → Executor → Replan → [End | back to Executor]`
- Planner runs once (expensive model), produces numbered checklist
- Executor runs each step (cheap model)
- **Replan node** inspects full execution history, decides: done or revise?
- Replanning is a **standard node**, not an exception case

**Triggers for replanning:**
- Step fails (tool error, test failure)
- Step result reveals plan was incomplete
- Step result contradicts an assumption

**Adopt pattern:** Explicit replan triggers (we call this "gap closure conditions")

---

## Cross-Framework Comparisons

### Task Structure

| Framework | Format | Verification Built-in? | File Paths Explicit? |
|-----------|--------|----------------------|---------------------|
| GSD | XML in Markdown | Yes (`<verify>`) | Yes (`<files>`) |
| Superpowers | Markdown checklist | Yes (runnable command) | Yes |
| BMAD | YAML + story files | Yes (acceptance criteria) | Yes |
| Feature-Dev | Blueprint prose | No | Yes |
| Devin | Numbered prose | No | No |
| Aider | In-context prose | No | No |
| LangChain | Numbered prose | No | No |

### Dependency Expression

| Framework | Model | Explicit? | Parallel Execution? |
|-----------|-------|-----------|-------------------|
| GSD | Wave number | Implicit | Yes |
| BMAD | `depends_on: [id]` | Explicit DAG | Via workflow engine |
| GAP | Edge set G=(V,E) | Explicit DAG | Yes |
| Feature-Dev | Ordered checklist | Sequential | No |
| LangChain | Sequential list | Sequential | No |

### Plan Mutability

| Framework | Mutability | Mechanism |
|-----------|-----------|-----------|
| GSD | Immutable + gap plans | New plans for gaps; existing never edited |
| Superpowers | Immutable | Written before execution; no replanning |
| Feature-Dev | Immutable | Blueprint once; executor follows |
| BMAD | Mutable (living docs) | Updated in-place by Scrum Master |
| Windsurf | Adaptive | Background planner updates continuously |
| Devin | Adaptive | Plan changes as execution proceeds |
| LangChain | Adaptive | Replan node triggered on failure |

---

## Synthesis: Recommendations for Our PLAN Phase

### Built-in Default Provider

```
1. Read specs:
   - Load product and tech specs for the feature
   - Load research findings for context

2. Produce architecture reasoning (separate from plan):
   - Decisions, trade-offs, build sequence
   - Preserved in research.md; plan in plan.md

3. Create wave-grouped plan in .spec/plan.md:

   ## Wave 1 (parallel — no dependencies)
   - [ ] T1: [description] | Files: `path` | Verify: `command` | Done: [criteria]
   - [ ] T2: [description] | Files: `path` | Verify: `command` | Done: [criteria]

   ## Wave 2 (parallel — depends on Wave 1)
   - [ ] T3: [description] [depends: T1, T2] | Files: `path` | Verify: `cmd`

   ## Plan Revision Conditions
   - If Wave N fails after 2 retries → stop, escalate
   - If file structure differs from spec → update spec first

4. Present to user: "Ready to implement? Changes?"
```

### Concrete Plan Format

Combines GSD verification-backed tasks + BMAD explicit dependencies + Superpowers dumb-executor explicitness:

```markdown
---
feature: user-authentication
milestone: M2-login-flow
updated: 2026-03-13
---

## Context
Implements login per tech-auth.md. Assumes User model from M1 complete.

## Wave 1 (parallel — no dependencies)

### T1: JWT config and secret loading
Files: `src/config/jwt.ts`
Action: Export jwtConfig with secret (from JWT_SECRET env, required), expirySeconds
  (default 3600), algorithm ('HS256'). Throw at startup if missing.
  Use jose (NOT jsonwebtoken — CommonJS/ESM conflict).
Verify: `pnpm typecheck && node -e "require('./src/config/jwt')"`
Done: TypeScript compiles. Node import succeeds.

### T2: Login request validation schema
Files: `src/schemas/auth.ts`
Action: Add Zod schema loginSchema with email (string, email) and password
  (string, min 8). Export inferred type LoginInput.
Verify: `pnpm test src/schemas/auth.test.ts`
Done: Schema tests pass. Invalid inputs rejected with descriptive errors.

## Wave 2 (depends on T1, T2)

### T3: Login route handler
Files: `src/routes/auth.ts`
Action: POST /auth/login. Parse with loginSchema (400 invalid). Find user by email
  (404 not found). bcrypt.compare (401 wrong). Issue JWT via jose SignJWT.
Verify: `pnpm test src/routes/auth.test.ts`
Done: All auth tests pass: happy path, invalid input, wrong password.

## Plan Revision Conditions
- Wave 1 failure after 1 retry → stop, escalate (likely env config)
- Wave 2 T3 failure after 2 retries → gap task targeting specific failure
- TypeScript error not caught by verify → T1-gap before proceeding
```

### Plugin Enhancements

| Plugin | Enhancement |
|--------|-------------|
| **feature-dev** | Spawn 2-3 competing `code-architect` agents with different mandates. Present proposals to user. Use when approach is genuinely uncertain. |
| **superpowers** | Enforce "dumb executor" explicitness — embed complete code in tasks, pre-write commit messages |

### Key Design Decisions

1. **Wave-based grouping (GSD).** Independent tasks in parallel waves. Derived from dependency topology.
2. **Plan immutability (GSD).** Once approved, plan doesn't change. Deviations trigger gap closure plans, not edits.
3. **Validation criteria per task (GSD + Superpowers).** Runnable `Verify` commands and measurable `Done` criteria.
4. **Self-contained tasks (Superpowers).** Exact file paths, what to avoid and why, runnable verification.
5. **Markdown format, not XML.** Checkboxes for tracking. Compatible with spec system.
6. **Explicit plan revision conditions (LangChain).** Name the conditions that trigger replanning — not assumed.
7. **Atomic tasks.** Max 3 tasks per wave. Each fits ~50% of fresh context. 2-15 minute execution.
8. **Vertical slices (GSD).** Group by end-to-end feature, not horizontal layer.
9. **Separate reasoning from planning (Aider).** Architecture blueprint in research.md, tasks in plan.md.
10. **Explicit dependencies alongside waves (BMAD).** `[depends: T1, T2]` for clarity beyond wave position.

---

## Sources

- [GSD Framework (GitHub)](https://github.com/gsd-build/get-shit-done)
- [GSD XML Plan Structure](https://zread.ai/gsd-build/get-shit-done/16-xml-plan-structure)
- [Beating context rot with GSD - The New Stack](https://thenewstack.io/beating-the-rot-and-getting-stuff-done/)
- [Feature-Dev code-architect](https://github.com/anthropics/claude-code/blob/main/plugins/feature-dev/agents/code-architect.md)
- [AI Agent Design Patterns — Azure](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns)
- [Superpowers (GitHub)](https://github.com/obra/superpowers)
- [Superpowers explained — Dev Genius](https://blog.devgenius.io/superpowers-explained-the-claude-plugin-that-enforces-tdd-subagents-and-planning-c7fe698c3b82)
- [BMAD Method (GitHub)](https://github.com/bmad-code-org/BMAD-METHOD)
- [BMAD Docs](https://docs.bmad-method.org/)
- [Windsurf Cascade docs](https://docs.windsurf.com/windsurf/cascade/cascade)
- [Devin 2.0 Technical Design](https://medium.com/@takafumi.endo/agent-native-development-a-deep-dive-into-devin-2-0s-technical-design-3451587d23c0)
- [OpenHands Plan Tool issue #9970](https://github.com/OpenHands/OpenHands/issues/9970)
- [Aider Architect mode](https://aider.chat/2024/09/26/architect.html)
- [GAP: Graph-based Agent Planning — NeurIPS 2025](https://arxiv.org/abs/2510.25320)
- [LangChain Plan-and-Execute Agents](https://blog.langchain.com/planning-agents/)
- [LangGraph Plan-and-Execute Tutorial](https://langchain-ai.github.io/langgraphjs/tutorials/plan-and-execute/plan-and-execute/)
- [Towards a science of scaling agent systems — Google Research](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)
