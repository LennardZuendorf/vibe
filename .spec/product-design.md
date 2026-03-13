---
type: branch
scope: product
parent: product.md
covers: functional design, workflow diagrams, user flows, interaction patterns
updated: 2026-03-13
---

# Engineering Agent — Functional Design

**Parent:** [product.md](product.md)

This document describes the functional design of the engineering agent framework: how users interact with it, how phases flow, and how plugins integrate.

---

## System Overview

```mermaid
graph TB
    subgraph "User Interface"
        U["/develop feature-x"]
        S["/setup-framework"]
        SP["/spec"]
    end

    subgraph "Orchestration Layer"
        O[Phase Orchestrator]
        C[".framework.json Config"]
        PG[Phase Gate Hooks]
    end

    subgraph "Phase Providers"
        BI[Built-in Defaults]
        SP_P[Superpowers Plugin]
        FD_P[Feature-Dev Plugin]
        SI_P[Simplify Plugin]
    end

    subgraph "Backbone"
        SPEC[".spec/ System"]
        PHASE[".phase State"]
        LESSONS["lessons.md"]
    end

    U --> O
    S --> C
    O --> C
    O --> PG
    PG --> PHASE
    O -->|routes to| BI
    O -->|routes to| SP_P
    O -->|routes to| FD_P
    O -->|routes to| SI_P
    BI --> SPEC
    SP_P --> SPEC
    FD_P --> SPEC
    SI_P --> SPEC
    SPEC --> LESSONS
```

---

## Core Workflow: `/develop`

The main user flow. Everything starts with `/develop <description>`.

```mermaid
flowchart TD
    START(["/develop feature-x"]) --> CHECK_PHASE{".phase exists?"}
    CHECK_PHASE -->|No| RESEARCH
    CHECK_PHASE -->|Yes| RESUME["Resume from saved phase"]
    RESUME --> RESEARCH & DISCUSS & SPEC & PLAN & IMPLEMENT & REVIEW

    subgraph "Phase 1: RESEARCH"
        RESEARCH[Research] --> R1["Read lessons.md"]
        R1 --> R2["Read existing specs"]
        R2 --> R3["Spawn parallel Explore agents"]
        R3 --> R4["Write findings to .spec/research/"]
        R4 --> R5["Present findings to user"]
    end

    R5 --> GATE1{{"User approves?"}}
    GATE1 -->|Yes| DISCUSS
    GATE1 -->|No| R3

    subgraph "Phase 2: DISCUSS"
        DISCUSS[Discuss] --> D1["Surface ambiguities"]
        D1 --> D2["Clarify scope & constraints"]
        D2 --> D3["Align on approach"]
    end

    D3 --> GATE2{{"User approves?"}}
    GATE2 -->|Yes| SPEC
    GATE2 -->|No| D1

    subgraph "Phase 3: SPEC"
        SPEC[Spec] --> S1["Write/update product spec"]
        S1 --> S2["Write/update tech spec"]
        S2 --> S3["Validate specs"]
        S3 --> S4["Present specs to user"]
    end

    S4 --> GATE3{{"User approves?"}}
    GATE3 -->|Yes| PLAN
    GATE3 -->|No| S1

    subgraph "Phase 4: PLAN"
        PLAN[Plan] --> P1["Read specs"]
        P1 --> P2["Group tasks into waves"]
        P2 --> P3["Set validation criteria"]
        P3 --> P4["Present plan to user"]
    end

    P4 --> GATE4{{"User approves?"}}
    GATE4 -->|Yes| IMPLEMENT
    GATE4 -->|No| P1

    subgraph "Phase 5: IMPLEMENT"
        IMPLEMENT[Implement] --> I1["Execute wave N"]
        I1 --> I2["Run tests in background"]
        I2 --> I3{{"Gap found?"}}
        I3 -->|Yes| I4["Update plan first"]
        I4 --> I1
        I3 -->|No| I5{{"More waves?"}}
        I5 -->|Yes| I1
        I5 -->|No| I6["All tasks complete"]
    end

    I6 --> REVIEW

    subgraph "Phase 6: REVIEW"
        REVIEW[Review] --> V1["Goal-backward verification"]
        V1 --> V2["Run test suite"]
        V2 --> V3["Run /simplify"]
        V3 --> V4["Self-review checklist"]
        V4 --> V5{{"Spec gaps found?"}}
        V5 -->|Yes| RESEARCH
        V5 -->|No| V6["Present results to user"]
    end

    V6 --> DONE([DONE])
```

---

## Setup Flow: `/setup-framework`

How users configure which plugins to use.

```mermaid
flowchart TD
    START(["/setup-framework"]) --> DETECT["Scan .agents/skills/ for known plugins"]
    DETECT --> SHOW["Display detected plugins"]
    SHOW --> ASK_RESEARCH["Research provider?<br/>• built-in (default)<br/>• feature-dev<br/>• superpowers"]
    ASK_RESEARCH --> ASK_DISCUSS["Discuss provider?<br/>• built-in (default)<br/>• superpowers"]
    ASK_DISCUSS --> ASK_PLAN["Plan provider?<br/>• built-in (default)<br/>• feature-dev"]
    ASK_PLAN --> ASK_IMPL["Implementation options?<br/>• TDD enforcement? (superpowers)<br/>• Parallel waves? (default: yes)"]
    ASK_IMPL --> ASK_REVIEW["Review provider?<br/>• simplify (default)<br/>• feature-dev<br/>• both"]
    ASK_REVIEW --> ASK_PATTERNS["Built-in patterns?<br/>☑ Wave grouping<br/>☑ Plan immutability<br/>☑ Gap closure<br/>☑ Pressure resistance"]
    ASK_PATTERNS --> GENERATE["Generate .framework.json"]
    GENERATE --> VALIDATE["Validate config"]
    VALIDATE --> DONE(["Config saved. Ready to /develop"])
```

---

## Plugin Routing: How Providers Are Selected

```mermaid
flowchart LR
    PHASE["Current Phase"] --> READ_CONFIG["Read .framework.json"]
    READ_CONFIG --> GET_PROVIDER["Get provider for this phase"]
    GET_PROVIDER --> CHECK{{"Provider installed?"}}
    CHECK -->|Yes| DELEGATE["Delegate to plugin skill"]
    CHECK -->|No| WARN["Warn: plugin not found"]
    WARN --> FALLBACK["Use built-in default"]
    DELEGATE --> EXECUTE["Execute phase"]
    FALLBACK --> EXECUTE
    EXECUTE --> WRITE_SPEC["Write output to .spec/"]
    WRITE_SPEC --> GATE{{"User approves?"}}
    GATE -->|Yes| NEXT["Advance to next phase"]
    GATE -->|No| EXECUTE
```

---

## Phase Gate Enforcement

How hooks prevent premature writes.

```mermaid
flowchart TD
    TOOL["Agent calls Edit/Write/NotebookEdit"] --> HOOK["PreToolUse hook fires"]
    HOOK --> READ_PHASE["Read .spec/.phase"]
    READ_PHASE --> CHECK{{"Phase allows writes?"}}

    CHECK -->|"RESEARCH"| BLOCK_R["❌ Block: only .spec/research/ allowed"]
    CHECK -->|"DISCUSS"| BLOCK_D["❌ Block: no file writes allowed"]
    CHECK -->|"SPEC"| ALLOW_S["✅ Allow: only .spec/*.md files"]
    CHECK -->|"PLAN"| ALLOW_P["✅ Allow: only .spec/plan*.md files"]
    CHECK -->|"IMPLEMENT"| ALLOW_I["✅ Allow: all files"]
    CHECK -->|"REVIEW"| ALLOW_FIX["✅ Allow: fixes only"]
```

---

## Wave-Based Implementation

How tasks are grouped and executed within the IMPLEMENT phase.

```mermaid
flowchart TD
    PLAN["Read plan.md"] --> GROUP["Group tasks by dependencies"]
    GROUP --> W1["Wave 1: Independent tasks"]
    GROUP --> W2["Wave 2: Depends on Wave 1"]
    GROUP --> W3["Wave 3: Depends on Wave 2"]

    W1 --> PAR1["Spawn parallel subagents"]
    PAR1 --> T1A["Task A (worktree)"]
    PAR1 --> T1B["Task B (worktree)"]
    PAR1 --> T1C["Task C (worktree)"]

    T1A --> MERGE1["Merge results"]
    T1B --> MERGE1
    T1C --> MERGE1

    MERGE1 --> TEST1["Background: run tests"]
    TEST1 --> W2

    W2 --> PAR2["Spawn parallel subagents"]
    PAR2 --> T2A["Task D"]
    PAR2 --> T2B["Task E"]

    T2A --> MERGE2["Merge results"]
    T2B --> MERGE2

    MERGE2 --> TEST2["Background: run tests"]
    TEST2 --> W3

    W3 --> FINAL["Final validation"]
```

---

## Session Resumption

How the framework handles interrupted work.

```mermaid
flowchart TD
    START(["/develop feature-x"]) --> CHECK{".phase file exists?"}
    CHECK -->|No| FRESH["Start at RESEARCH"]
    CHECK -->|Yes| READ["Read .phase"]
    READ --> LOAD["Load relevant specs + plan"]
    LOAD --> STATUS["Present status:<br/>'Resuming from IMPLEMENT.<br/>Last: M2. Next: M3.'"]
    STATUS --> CONTINUE["Continue from saved phase"]

    FRESH --> SAVE1["Write RESEARCH to .phase"]
    CONTINUE --> SAVE2["Phase transitions update .phase"]
```

---

## Interaction Patterns Per Phase

### RESEARCH: Agent spawns research, user confirms

```
User: /develop "Add dark mode toggle"

Agent: Spawning research agents...
  → Agent 1: Searching for existing theme/color code
  → Agent 2: Searching for related tests
  → Agent 3: Searching for UI component patterns

Agent: Research complete. Here's what I found:
  **Exists:** ThemeContext at src/contexts/theme.tsx
  **Must build:** Toggle component, CSS variables, persistence
  **Risk:** Existing components use hardcoded colors (HIGH - verified)

  Does this match your understanding? Corrections?

User: Looks right, but we also need to check the design tokens file.

Agent: [updates research, re-presents]

User: Good, move on.
```

### DISCUSS: Structured clarification

```
Agent: Before I write specs, let me clarify a few things:

  1. Scope: Should dark mode apply to the entire app or just the main content area?
  2. Persistence: Local storage, user preferences API, or system preference?
  3. Transition: Instant toggle or animated transition?
  4. Default: Start in light mode, or follow OS preference?

User: Entire app, local storage, instant, follow OS.

Agent: Clear. Moving to specs.
```

### SPEC: Structured writing, user reviews

```
Agent: I've written the specs:

  product-dark-mode.md: What users see — toggle location, behavior,
    which surfaces change, accessibility requirements.

  tech-dark-mode.md: How it works — CSS custom properties, ThemeContext
    updates, localStorage key, component migration strategy.

  Specs validated ✓. Want to review them before I plan?
```

### PLAN: Wave-grouped tasks, user approves

```
Agent: Implementation plan:

  Wave 1 (independent):
    □ Add CSS custom properties for all color tokens
    □ Add dark mode toggle component
    □ Add localStorage persistence hook

  Wave 2 (depends on Wave 1):
    □ Update ThemeContext to use custom properties
    □ Wire toggle to ThemeContext

  Wave 3 (depends on Wave 2):
    □ Migrate existing components from hardcoded to token colors
    □ Add prefers-color-scheme media query

  Ready to implement? Changes?
```

---

## State Machine

```mermaid
stateDiagram-v2
    [*] --> RESEARCH: /develop
    RESEARCH --> DISCUSS: user approves findings
    DISCUSS --> SPEC: scope clarified
    SPEC --> PLAN: specs approved
    PLAN --> IMPLEMENT: plan approved
    IMPLEMENT --> REVIEW: all tasks done
    REVIEW --> DONE: review clean
    REVIEW --> RESEARCH: spec gaps found
    DONE --> [*]

    RESEARCH --> RESEARCH: user requests more research
    DISCUSS --> DISCUSS: more questions
    SPEC --> SPEC: user requests changes
    PLAN --> PLAN: user requests changes
    IMPLEMENT --> IMPLEMENT: gap closure (update plan, continue)
```
