---
type: branch
scope: product
parent: product.md
covers: functional design, workflow diagrams, user flows, interaction patterns
updated: 2026-03-14
---

# Engineering Agent — Functional Design

**Parent:** [product.md](product.md)

This document describes the functional design of the engineering agent framework: how the two clusters work, how global and feature specs interact, and how plugins integrate.

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
        O[Cluster Orchestrator]
        C[".framework.json Config"]
        PG[Phase Gate Hooks]
    end

    subgraph "Phase Providers"
        BI[Built-in Defaults]
        SP_P[Superpowers Plugin]
        FD_P[Feature-Dev Plugin]
        SI_P[Simplify Plugin]
        CE_P[Compound Engineering Plugin]
    end

    subgraph "Spec System"
        GS["Global Specs<br/>.spec/product.md<br/>.spec/tech.md"]
        FS["Feature Specs<br/>.spec/features/&lt;name&gt;/"]
        LESSONS["lessons.md"]
        PHASE[".phase State"]
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
    O -->|routes to| CE_P
    BI --> GS & FS
    SP_P --> GS & FS
    FD_P --> GS & FS
    SI_P --> GS & FS
    CE_P --> LESSONS
    FS -->|merge after ship| GS
```

---

## Core Workflow: `/develop`

The main user flow. Two clusters with a handoff gate between them.

```mermaid
flowchart TD
    START(["/develop feature-x"]) --> CHECK_PHASE{".phase exists?"}
    CHECK_PHASE -->|No| DESIGN_CLUSTER
    CHECK_PHASE -->|Yes| RESUME["Resume from saved phase"]

    subgraph DESIGN_CLUSTER ["DESIGN CLUSTER — Front-loaded Thinking"]
        direction TB

        RESEARCH["RESEARCH"] --> R1["Read lessons.md + global specs"]
        R1 --> R2["Spawn parallel Explore agents"]
        R2 --> R3["Write findings to .spec/research/"]
        R3 --> R4["Present findings"]

        R4 --> DISCUSS["DISCUSS"]
        DISCUSS --> D1["Surface ambiguities"]
        D1 --> D2["Clarify scope & constraints"]
        D2 --> D3["Align on approach"]

        D3 --> SPEC["SPEC"]
        SPEC --> S1["Create .spec/features/&lt;name&gt;/"]
        S1 --> S2["Write feature product spec"]
        S2 --> S3["Write feature tech spec"]
        S3 --> S4["Update global specs if cross-cutting"]
        S4 --> S5["Validate all specs"]

        S5 --> PLAN["PLAN"]
        PLAN --> P1["Read feature specs"]
        P1 --> P2["Group tasks into waves"]
        P2 --> P3["Set validation criteria"]
        P3 --> P4["Write feature plan"]
    end

    P4 --> DESIGN_GATE{{"User approves design?"}}
    DESIGN_GATE -->|No| RESEARCH
    DESIGN_GATE -->|Yes| IMPL_CLUSTER

    subgraph IMPL_CLUSTER ["IMPLEMENTATION CLUSTER — Execution"]
        direction TB

        VERIFY["VERIFY"] --> V1["Scan codebase against feature spec"]
        V1 --> V2{{"Spec still valid?"}}
        V2 -->|Yes| V3["Confirm: ready to implement"]
        V2 -->|Drift detected| V4["Flag changes, amend spec"]
        V4 --> V3

        V3 --> IMPLEMENT["IMPLEMENT"]
        IMPLEMENT --> I1["Execute wave N"]
        I1 --> I2["Run tests in background"]
        I2 --> I3{{"More waves?"}}
        I3 -->|Yes| I1
        I3 -->|No| I4["All tasks complete"]

        I4 --> REVIEW["REVIEW"]
        REVIEW --> RV1["Goal-backward verification"]
        RV1 --> RV2["Run test suite"]
        RV2 --> RV3["Run /simplify"]
        RV3 --> RV4["Self-review checklist"]

        RV4 --> LEARN["LEARN"]
        LEARN --> L1["Extract session learnings"]
        L1 --> L2["Update lessons.md"]
        L2 --> L3["Prune stale lessons"]
        L3 --> L4["Merge feature spec → global specs"]
        L4 --> L5["Archive feature spec directory"]
    end

    RV4 -->|"Spec gaps found"| V1
    L5 --> DONE([DONE])
```

---

## Global vs Feature Spec Lifecycle

How feature specs are created, used, and merged.

```mermaid
flowchart LR
    subgraph "Design Cluster"
        READ_GLOBAL["Read global specs"] --> CREATE_FEATURE["Create .spec/features/dark-mode/"]
        CREATE_FEATURE --> WRITE_PRODUCT["Write feature product.md"]
        WRITE_PRODUCT --> WRITE_TECH["Write feature tech.md"]
        WRITE_TECH --> WRITE_PLAN["Write feature plan.md"]
    end

    subgraph "Implementation Cluster"
        VERIFY_SPEC["VERIFY: scan codebase"] --> READ_FEATURE["Read feature specs<br/>(read-only)"]
        READ_FEATURE --> IMPLEMENT_CODE["IMPLEMENT"]
        IMPLEMENT_CODE --> REVIEW_CODE["REVIEW"]
    end

    subgraph "Merge (LEARN Phase)"
        EXTRACT["Extract cross-cutting decisions"] --> UPDATE_GLOBAL["Update global product.md / tech.md"]
        UPDATE_GLOBAL --> ARCHIVE["Archive .spec/features/dark-mode/"]
    end

    WRITE_PLAN --> VERIFY_SPEC
    REVIEW_CODE --> EXTRACT
```

### Feature Spec Directory Structure

```
.spec/
├── product.md                          # GLOBAL: project-wide product spec
├── tech.md                             # GLOBAL: project-wide tech spec
├── product-design.md                   # GLOBAL: project-wide design
├── lessons.md                          # GLOBAL: accumulated learnings
├── plan.md                             # GLOBAL: overall roadmap
│
├── features/
│   ├── dark-mode/                      # FEATURE: ephemeral during development
│   │   ├── product.md                  #   What dark mode does (user experience)
│   │   ├── tech.md                     #   How dark mode is built (architecture)
│   │   ├── plan.md                     #   Implementation waves + tasks
│   │   └── research.md                 #   Research findings (codebase scan)
│   │
│   └── auth-flow/                      # Another feature in parallel
│       ├── product.md
│       ├── tech.md
│       └── plan.md
│
├── archive/                            # Archived feature specs (post-merge)
│   └── dark-mode/                      #   Kept for history, not loaded
│       └── ...
```

### What Gets Merged vs Archived

| Content | Action | Example |
|---------|--------|---------|
| **Cross-cutting architecture decisions** | Merge into `tech.md` | "We use CSS custom properties for theming" |
| **New design patterns** | Merge into `product-design.md` or `tech.md` | "Toggle components follow X pattern" |
| **Feature-specific implementation detail** | Archive only | "Dark mode uses localStorage key `theme-pref`" |
| **Lessons learned** | Already in `lessons.md` | Captured during LEARN phase |
| **Feature product requirements** | Archive only (they're done) | "Toggle appears in settings panel" |

---

## The VERIFY Phase

The key insight: **pre-implementation research on the codebase is necessary, but re-doing spec writing is not.** VERIFY is a lightweight check, not a repeat of the Design Cluster.

```mermaid
flowchart TD
    START["VERIFY phase begins"] --> SCAN["Scan codebase for changes<br/>since Design Cluster completed"]

    SCAN --> CHECK_FILES{{"Referenced files<br/>still exist?"}}
    CHECK_FILES -->|All present| CHECK_INTERFACES
    CHECK_FILES -->|Files moved/deleted| FLAG1["⚠ Flag: file paths changed"]

    CHECK_INTERFACES{{"Interfaces/APIs<br/>match spec?"}}
    CHECK_INTERFACES -->|Match| CHECK_DEPS
    CHECK_INTERFACES -->|Changed| FLAG2["⚠ Flag: interface drift"]

    CHECK_DEPS{{"Dependencies<br/>still available?"}}
    CHECK_DEPS -->|Yes| PASS["✅ Spec verified — proceed to IMPLEMENT"]
    CHECK_DEPS -->|Changed| FLAG3["⚠ Flag: dependency change"]

    FLAG1 --> AMEND["Amend feature spec<br/>(targeted fix, not rewrite)"]
    FLAG2 --> AMEND
    FLAG3 --> AMEND
    AMEND --> PASS
```

**What VERIFY does:**
- Scans codebase for changes since the Design Cluster completed
- Checks that file paths referenced in the feature spec still exist
- Checks that interfaces/APIs the feature depends on haven't changed
- Checks that dependencies are still available and compatible

**What VERIFY does NOT do:**
- Rewrite the product spec
- Rewrite the tech spec
- Re-run the full research phase
- Re-discuss scope with the user (unless drift is major)

---

## Setup Flow: `/setup-framework`

How users configure which plugins to use.

```mermaid
flowchart TD
    START(["/setup-framework"]) --> DETECT["Scan .agents/skills/ for known plugins"]
    DETECT --> SHOW["Display detected plugins"]

    SHOW --> ASK_DESIGN["Design Cluster providers?"]
    ASK_DESIGN --> ASK_RESEARCH["Research: built-in / feature-dev / superpowers"]
    ASK_RESEARCH --> ASK_DISCUSS["Discuss: built-in / superpowers"]
    ASK_DISCUSS --> ASK_PLAN["Plan: built-in / feature-dev"]

    ASK_PLAN --> ASK_IMPL["Implementation Cluster providers?"]
    ASK_IMPL --> ASK_IMPL_OPTS["Implement: TDD? (superpowers) / Parallel waves? (yes)"]
    ASK_IMPL_OPTS --> ASK_REVIEW["Review: simplify / feature-dev / both"]
    ASK_REVIEW --> ASK_LEARN["Learn: built-in / compound-engineering"]

    ASK_LEARN --> ASK_PATTERNS["Built-in patterns?<br/>☑ Wave grouping<br/>☑ Gap closure<br/>☑ Pressure resistance<br/>☑ Spec verification"]
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
    EXECUTE --> WRITE_SPEC["Write output to feature spec"]
    WRITE_SPEC --> NEXT["Advance to next phase"]
```

---

## Phase Gate Enforcement

How hooks prevent premature writes. Gates enforce cluster boundaries and phase-appropriate writes.

```mermaid
flowchart TD
    TOOL["Agent calls Edit/Write/NotebookEdit"] --> HOOK["PreToolUse hook fires"]
    HOOK --> READ_PHASE["Read .spec/.phase"]
    READ_PHASE --> CHECK{{"Phase allows this write?"}}

    CHECK -->|"RESEARCH"| ALLOW_R["✅ Allow: .spec/features/*/research.md, .spec/research/"]
    CHECK -->|"DISCUSS"| BLOCK_D["❌ Block: no file writes"]
    CHECK -->|"SPEC"| ALLOW_S["✅ Allow: .spec/features/*/*.md, .spec/*.md"]
    CHECK -->|"PLAN"| ALLOW_P["✅ Allow: .spec/features/*/plan.md, .spec/plan*.md"]
    CHECK -->|"VERIFY"| ALLOW_V["✅ Allow: .spec/features/*/*.md (amendments only)"]
    CHECK -->|"IMPLEMENT"| ALLOW_I["✅ Allow: all files"]
    CHECK -->|"REVIEW"| ALLOW_FIX["✅ Allow: fixes only"]
    CHECK -->|"LEARN"| ALLOW_L["✅ Allow: .spec/*.md, lessons.md, .spec/archive/"]
```

---

## Wave-Based Implementation

How tasks are grouped and executed within the IMPLEMENT phase. Unchanged from previous design — waves are a proven pattern.

```mermaid
flowchart TD
    PLAN["Read feature plan.md"] --> GROUP["Group tasks by dependencies"]
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

How the framework handles interrupted work. The `.phase` file now encodes both cluster and phase.

```mermaid
flowchart TD
    START(["/develop feature-x"]) --> CHECK{".phase file exists?"}
    CHECK -->|No| FRESH["Start at DESIGN:RESEARCH"]
    CHECK -->|Yes| READ["Read .phase"]
    READ --> WHICH_CLUSTER{{"Which cluster?"}}

    WHICH_CLUSTER -->|DESIGN| LOAD_GLOBAL["Load global specs + research"]
    WHICH_CLUSTER -->|IMPL| LOAD_FEATURE["Load feature specs + plan"]

    LOAD_GLOBAL --> STATUS_D["'Resuming Design Cluster.<br/>Phase: SPEC. Feature: dark-mode.'"]
    LOAD_FEATURE --> STATUS_I["'Resuming Implementation.<br/>Last wave: 2. Next: 3.'"]

    STATUS_D --> CONTINUE["Continue from saved phase"]
    STATUS_I --> CONTINUE

    FRESH --> SAVE["Write DESIGN:RESEARCH to .phase"]
```

### Phase File Format

```
# .spec/.phase
DESIGN:RESEARCH          # Currently in Design Cluster, RESEARCH phase
DESIGN:DISCUSS           # Currently in Design Cluster, DISCUSS phase
DESIGN:SPEC              # Currently in Design Cluster, SPEC phase
DESIGN:PLAN              # Currently in Design Cluster, PLAN phase
IMPL:VERIFY              # Currently in Implementation Cluster, VERIFY phase
IMPL:IMPLEMENT           # Currently in Implementation Cluster, IMPLEMENT phase
IMPL:REVIEW              # Currently in Implementation Cluster, REVIEW phase
IMPL:LEARN               # Currently in Implementation Cluster, LEARN phase
DONE                     # Feature complete
```

---

## Interaction Patterns

### Design Cluster — Dense, Collaborative

The Design Cluster is a tight conversation between user and agent. Phases flow quickly because they're all about thinking, not coding.

```
User: /develop "Add dark mode toggle"

Agent: [RESEARCH] Scanning codebase + reading global specs...
  → Found: ThemeContext at src/contexts/theme.tsx
  → Found: Design tokens in src/styles/tokens.ts
  → Must build: Toggle component, CSS variables, persistence
  → Risk: Existing components use hardcoded colors (HIGH)

  [DISCUSS] A few questions before I write the feature spec:
  1. Should dark mode apply to the entire app or just the main content area?
  2. Persistence: localStorage, user preferences API, or system preference?
  3. Default: light mode, or follow OS preference?

User: Entire app, localStorage, follow OS.

Agent: [SPEC] Writing feature specs to .spec/features/dark-mode/...
  ✓ product.md — what users experience
  ✓ tech.md — CSS custom properties, ThemeContext updates, migration strategy
  ✓ Updated global tech.md — added theming section to architecture overview

  [PLAN] Implementation plan:
  Wave 1: CSS custom properties + toggle component + persistence hook
  Wave 2: Wire ThemeContext + toggle integration
  Wave 3: Migrate hardcoded colors + prefers-color-scheme

  Ready to implement? Any changes to the design?

User: Looks good. Go.
```

### Implementation Cluster — Autonomous, Verify-First

The Implementation Cluster is mostly autonomous. The agent verifies, implements, reviews, and learns. User is notified at completion.

```
Agent: [VERIFY] Checking feature spec against current codebase...
  ✓ ThemeContext still at src/contexts/theme.tsx
  ✓ Design tokens still at src/styles/tokens.ts
  ✓ No interface changes since design
  Spec verified — proceeding to implement.

  [IMPLEMENT]
  Wave 1: Spawning 3 parallel agents...
    ✓ CSS custom properties added
    ✓ Toggle component created
    ✓ Persistence hook created
  Tests: 42 passed, 0 failed

  Wave 2: Spawning 2 parallel agents...
    ✓ ThemeContext updated
    ✓ Toggle wired
  Tests: 45 passed, 0 failed

  Wave 3: Migrating hardcoded colors...
    ✓ 12 components migrated
    ✓ prefers-color-scheme added
  Tests: 48 passed, 0 failed

  [REVIEW] Verifying against feature spec...
  ✓ All requirements met
  ✓ Tests passing
  ✓ /simplify found 1 suggestion (applied)
  ✓ No spec gaps

  [LEARN] Extracting learnings...
  + Added lesson: "Check for hardcoded colors before theming work"
  - Pruned stale lesson: "Use old color system" (superseded)
  → Merged into global tech.md: CSS custom properties as theming standard
  → Archived .spec/features/dark-mode/ to .spec/archive/dark-mode/

  Feature complete. ✓
```

---

## State Machine

```mermaid
stateDiagram-v2
    [*] --> DESIGN_RESEARCH: /develop

    state "Design Cluster" as DC {
        DESIGN_RESEARCH --> DESIGN_DISCUSS: findings confirmed
        DESIGN_DISCUSS --> DESIGN_SPEC: scope clarified
        DESIGN_SPEC --> DESIGN_PLAN: specs written
        DESIGN_RESEARCH --> DESIGN_RESEARCH: user requests more
        DESIGN_DISCUSS --> DESIGN_DISCUSS: more questions
        DESIGN_SPEC --> DESIGN_SPEC: user requests changes
        DESIGN_PLAN --> DESIGN_PLAN: user requests changes
    }

    state "Implementation Cluster" as IC {
        IMPL_VERIFY --> IMPL_IMPLEMENT: spec verified
        IMPL_IMPLEMENT --> IMPL_REVIEW: all waves done
        IMPL_REVIEW --> IMPL_LEARN: review clean
        IMPL_REVIEW --> IMPL_VERIFY: spec gaps found
        IMPL_IMPLEMENT --> IMPL_IMPLEMENT: wave progression
        IMPL_VERIFY --> IMPL_VERIFY: amending spec
    }

    DESIGN_PLAN --> IMPL_VERIFY: user approves design
    IMPL_LEARN --> DONE: learnings captured + merged

    DONE --> [*]
```

---

## Parallel Feature Development

Because feature specs are isolated in their own directories, multiple features can be designed and implemented in parallel without conflicts.

```mermaid
flowchart LR
    subgraph "Feature A: dark-mode"
        A_DESIGN["Design Cluster"] --> A_IMPL["Implementation Cluster"]
        A_IMPL --> A_MERGE["Merge → global"]
    end

    subgraph "Feature B: auth-flow"
        B_DESIGN["Design Cluster"] --> B_IMPL["Implementation Cluster"]
        B_IMPL --> B_MERGE["Merge → global"]
    end

    A_MERGE --> GLOBAL["Global Specs<br/>(updated sequentially)"]
    B_MERGE --> GLOBAL

    style GLOBAL fill:#f9f,stroke:#333
```

**Conflict resolution:** Merges into global specs happen sequentially. If Feature B's merge conflicts with changes Feature A already merged, the LEARN phase flags it and the user resolves.
