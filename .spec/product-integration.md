---
type: branch
scope: product
parent: product.md
covers: framework integration map, plugin routing, build-vs-reuse analysis, dependency chart
updated: 2026-03-14
---

# Engineering Agent — Framework Integration Map

**Parent:** [product.md](product.md)
**Sibling:** [product-design.md](product-design.md)

How we mix and match existing skills, plugins, and frameworks across the two clusters. What we reuse, what we build, and what we depend on.

---

## The Integration Problem

We have 8 phases across 2 clusters. For each phase, we need a **built-in default** that works with zero plugins, and optional **plugin providers** that can replace or enhance the default. The question is: what already exists that we can route to, and what do we need to build?

---

## Master Integration Chart

### Design Cluster (Bootstrap — runs once)

| Phase | Built-in Default | Plugin: superpowers | Plugin: feature-dev | Plugin: compound-eng | What We Build |
|-------|-----------------|--------------------|--------------------|---------------------|---------------|
| **RESEARCH** | Parallel Explore agents (Glob + Grep + WebSearch) | — | `code-explorer` agents with dependency graphs | — | Routing in `/develop` SKILL.md. Built-in already works via Agent tool. |
| **DISCUSS** | AskUserQuestion with structured prompts | `/superpowers:brainstorm` (Socratic diverge→converge) | — | — | **New: discussion prompt templates.** Built-in needs structured question sets per feature type. |
| **SPEC** | `/spec` skill (always, not pluggable) | — | — | — | Nothing. `/spec` exists and is complete. |
| **PLAN** | Wave-grouped plan writing (GSD patterns) | — | `code-architect` agents (competing proposals) | — | **New: global plan template** that sequences features. Current plan.md template is single-feature. |

### Implementation Cluster (Per-feature — repeats)

| Phase | Built-in Default | Plugin: superpowers | Plugin: feature-dev | Plugin: compound-eng | What We Build |
|-------|-----------------|--------------------|--------------------|---------------------|---------------|
| **VERIFY** | Codebase scan against feature spec (file paths, interfaces, deps) | — | `code-explorer` for targeted scan | — | **New: verify logic.** This phase doesn't exist yet. Need script + SKILL.md section. |
| **IMPLEMENT** | Wave-based subagent execution (GSD patterns) | TDD enforcement (red-green-refactor per task) | — | — | Routing in `/develop` SKILL.md. Built-in already works via Agent tool with worktrees. |
| **REVIEW** | `/simplify` (3 parallel review agents) | Dual-stage review (spec compliance + quality) | `code-reviewer` with confidence scoring | — | Nothing for default. `/simplify` exists. **New: adapter for chaining review providers.** |
| **LEARN** | Update lessons.md, prune stale, merge feature→global | — | — | `/ce:compound` (wiki-based learning) | **New: entire phase.** Built-in extractor, pruner, merger. Phase gate rules. |

---

## What Already Exists (Reuse As-Is)

| Component | Location | Status | Used In |
|-----------|----------|--------|---------|
| `/spec` skill | `.agents/skills/spec/` | Complete | SPEC phase (always) |
| `/simplify` skill | Available as skill | Complete | REVIEW phase (default provider) |
| Spec templates | `.agents/skills/spec/reference/templates/` | Complete | SPEC phase |
| Spec validation | `.agents/skills/spec/scripts/validate.sh` | Complete | SPEC, REVIEW, LEARN phases |
| Phase gate hook | `.claude/hooks/check-phase.sh` | Partial | All phases |
| Session start hook | `.claude/settings.json` | Complete | Session initialization |
| Phase gate script | `.agents/skills/develop/scripts/phase-gate.sh` | Complete | Hook enforcement |

**These are our hard dependencies.** The framework cannot function without `/spec` and the phase gate hook. `/simplify` is the default review provider but has a built-in fallback.

---

## What We Build New

### P0 — Framework Cannot Work Without These

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| **Config reader** | Logic in SKILL.md | `.agents/skills/develop/SKILL.md` | Read `.framework.json`, route phases to providers |
| **VERIFY phase** | New phase | `.agents/skills/develop/SKILL.md` | Pre-implementation spec check. Scan codebase, diff against feature spec, flag drift. |
| **LEARN phase** | New phase | `.agents/skills/develop/SKILL.md` | Post-review learning. Extract lessons, prune stale, merge feature→global, archive feature spec. |
| **Feature spec directories** | Convention | `.spec/features/<name>/` | Product.md + tech.md per feature. Creation during SPEC, consumption during VERIFY+IMPLEMENT, archive during LEARN. |
| **Global plan template** | Template | `.agents/skills/spec/reference/templates/plan-global.md` | Plan template that sequences multiple features with cross-feature dependencies. |
| **Phase file update** | Format change | `.spec/.phase` | New format: `CLUSTER:PHASE:FEATURE` (e.g., `IMPL:VERIFY:dark-mode`) |
| **Hook updates** | Script changes | `.claude/hooks/check-phase.sh` | Add rules for VERIFY, LEARN, DISCUSS phases. Support new `.phase` format. |

### P1 — Framework Works Without These, But Is Limited

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| **`/setup-framework` skill** | New skill | `.agents/skills/setup-framework/` | Interactive plugin detection, config generation |
| **Plugin detection script** | Shell script | `.agents/skills/setup-framework/scripts/detect-plugins.sh` | Scan `.agents/skills/` for known plugin signatures |
| **Config generator script** | Shell script | `.agents/skills/setup-framework/scripts/generate-config.sh` | Write `.framework.json` from user choices |
| **Discussion templates** | Prompt templates | `.agents/skills/develop/prompts/discuss-*.md` | Structured question sets for the DISCUSS phase by feature type |

### P2 — Plugin Adapters (Only If Plugins Are Installed)

| Component | Type | Location | Purpose |
|-----------|------|----------|---------|
| **Superpowers adapter** | Routing logic | In `/develop` SKILL.md | Map DISCUSS→brainstorm, IMPLEMENT→TDD, REVIEW→dual-stage |
| **Feature-dev adapter** | Routing logic | In `/develop` SKILL.md | Map RESEARCH→code-explorer, PLAN→code-architect, REVIEW→code-reviewer |
| **Compound-eng adapter** | Routing logic | In `/develop` SKILL.md | Map LEARN→/ce:compound |

---

## Plugin Dependency Matrix

Which plugins depend on what, and what they provide.

```
                         ┌─────────────────────────────────────────────┐
                         │          ENGINEERING AGENT CORE             │
                         │                                            │
                         │  /spec (always)                            │
                         │  /develop (orchestrator)                   │
                         │  phase-gate hooks                          │
                         │  .framework.json config                    │
                         │                                            │
                         │  Built-in defaults for ALL phases          │
                         └──────┬──────┬──────┬──────┬───────────────┘
                                │      │      │      │
                    ┌───────────┘      │      │      └────────────┐
                    │                  │      │                   │
              ┌─────▼─────┐    ┌──────▼──────▼──┐    ┌──────────▼──────────┐
              │ simplify  │    │  superpowers    │    │    feature-dev      │
              │           │    │                 │    │                     │
              │ REVIEW    │    │ DISCUSS         │    │ RESEARCH            │
              │ (default) │    │ IMPLEMENT (TDD) │    │ PLAN                │
              │           │    │ REVIEW          │    │ REVIEW              │
              │ Bundled,  │    │                 │    │                     │
              │ no setup  │    │ Optional        │    │ Optional            │
              └───────────┘    └─────────────────┘    └─────────────────────┘

                                                      ┌─────────────────────┐
                                                      │ compound-engineering│
                                                      │                     │
                                                      │ LEARN               │
                                                      │                     │
                                                      │ Optional, future    │
                                                      └─────────────────────┘
```

### Hard Dependencies (framework breaks without these)

| Dependency | Why | Substitutable? |
|-----------|-----|----------------|
| Claude Code CLI | Skills, hooks, subagents, worktrees — the runtime | No |
| `/spec` skill | Spec writing, validation, templates — the backbone | No |
| Phase gate hook (`check-phase.sh`) | Prevents writes during wrong phases | No |
| `.spec/` directory | All state lives here | No |
| Bash | All scripts, hooks, detection | No |

### Soft Dependencies (framework works without, but enhanced with)

| Dependency | What It Adds | Fallback Without It |
|-----------|-------------|---------------------|
| `/simplify` | 3-agent parallel code review | Self-review checklist in `/develop` |
| `jq` | Clean JSON parsing of `.framework.json` | Bash grep/sed parsing (uglier, fragile) |
| superpowers plugin | Brainstorming, TDD, dual-stage review | Built-in AskUserQuestion, wave-based implementation, `/simplify` |
| feature-dev plugin | Code-explorer, code-architect, code-reviewer agents | Built-in Explore agents, direct plan writing, `/simplify` |
| compound-engineering plugin | Wiki-based compound learning | Built-in lessons.md update |

---

## Phase Provider Routing — Full Decision Table

When `/develop` enters a phase, it reads `.framework.json` to decide who handles it.

### Design Cluster Phases

**RESEARCH:**
```
IF provider = "feature-dev" AND feature-dev installed:
  → Spawn feature-dev code-explorer agents
  → They return: dependency graph, related code, test coverage map
  → Write to .spec/research/ (or .spec/features/<name>/research.md)

IF provider = "built-in" (default):
  → Spawn 3-4 parallel Explore agents:
    Agent 1: Search for code related to ALL planned features (Glob + Grep)
    Agent 2: Search for tests, fixtures, examples
    Agent 3: Search for config, types, interfaces
    Agent 4: WebSearch for external APIs/libraries if relevant
  → Write consolidated findings to .spec/research/
```

**DISCUSS:**
```
IF provider = "superpowers" AND superpowers installed:
  → Invoke /superpowers:brainstorm
  → Socratic diverge→converge: generate options, pressure-test, commit
  → Covers ALL features in scope

IF provider = "built-in" (default):
  → For each feature, present structured questions:
    1. Scope boundaries (what's in, what's out)
    2. Constraints (tech, time, compatibility)
    3. Ambiguities (what could this mean?)
    4. Dependencies (what blocks what?)
  → Use AskUserQuestion with multi-select where appropriate
```

**SPEC:**
```
ALWAYS provider = "spec" (not configurable):
  → Write/update global product.md, tech.md
  → For each feature: create .spec/features/<name>/product.md, tech.md
  → Run validate.sh
```

**PLAN:**
```
IF provider = "feature-dev" AND feature-dev installed:
  → Spawn competing code-architect agents
  → Each proposes a different implementation approach
  → Present options to user, select best
  → Write selected plan to .spec/plan.md (global, multi-feature)

IF provider = "built-in" (default):
  → Read all feature specs
  → Sequence features by dependency
  → Group tasks into waves within each feature
  → Write to .spec/plan.md with cross-feature dependency map
```

### Implementation Cluster Phases

**VERIFY:**
```
ALWAYS provider = "built-in" (not pluggable):
  → Read .spec/features/<current-feature>/product.md and tech.md
  → Spawn Explore agent to scan codebase:
    - Check file paths referenced in spec still exist
    - Check interfaces/APIs haven't changed
    - Check dependencies still available
    - Diff against what was true during bootstrap
  → IF drift found: amend feature spec (targeted, not rewrite)
  → IF major drift: flag to user, may need mini Design Cluster
```

**IMPLEMENT:**
```
IF provider = "superpowers" AND superpowers installed:
  → TDD enforcement: for each task in wave:
    1. Write failing test first
    2. Write minimum code to pass
    3. Refactor
  → Still uses wave-based parallelism (GSD pattern)

IF provider = "built-in" (default):
  → Read current feature's section from plan.md
  → Execute waves sequentially, tasks within waves in parallel:
    - Spawn subagents with worktree isolation per task
    - Each agent gets: task description + feature spec (read-only)
    - Background test runs between waves
  → Update plan.md progress after each wave
```

**REVIEW:**
```
IF provider = "simplify" (default):
  → Invoke /simplify
  → 3 parallel review agents: Reuse, Quality, Efficiency
  → Apply suggestions

IF provider = "superpowers" AND superpowers installed:
  → Dual-stage review:
    Stage 1: Spec compliance (does code match feature spec?)
    Stage 2: Quality review (patterns, performance, security)

IF provider = "feature-dev" AND feature-dev installed:
  → Code-reviewer agents with confidence scoring
  → Per-file confidence: HIGH/MEDIUM/LOW
  → Focus manual review on LOW-confidence files

IF provider = "simplify+feature-dev" (chained):
  → Run feature-dev code-reviewer first (identifies problem areas)
  → Run /simplify second (deep review on flagged areas)

ALWAYS (regardless of provider):
  → Goal-backward verification against feature spec
  → Run test suite
  → Self-review checklist
```

**LEARN:**
```
IF provider = "compound-engineering" AND compound-eng installed:
  → Invoke /ce:compound
  → Writes to wiki-style documentation
  → THEN: built-in merger runs to sync wiki → lessons.md + global specs

IF provider = "built-in" (default):
  → Step 1: Scan session for learnings
    - User corrections during implementation
    - Test failures and their fixes
    - Spec drift found during VERIFY
    - Review findings from REVIEW
  → Step 2: Update lessons.md
    - Add new entries (Pattern/Rule/Category/Confidence/Date)
    - Prune entries whose referenced files no longer exist
  → Step 3: Merge feature spec → global specs
    - Cross-cutting architecture decisions → tech.md
    - New design patterns → product-design.md or tech.md
    - Feature-specific details → archive only
  → Step 4: Archive feature spec
    - Move .spec/features/<name>/ → .spec/archive/<name>/
  → Step 5: Update plan.md
    - Mark feature as complete
    - Note any downstream impacts for next feature
```

---

## Development Flow Configurations

### Configuration 1: Zero Plugins (Default)

```json
{
  "version": 1,
  "phases": {
    "research":  { "provider": "built-in" },
    "discuss":   { "provider": "built-in" },
    "spec":      { "provider": "spec" },
    "plan":      { "provider": "built-in" },
    "verify":    { "provider": "built-in" },
    "implement": { "provider": "built-in" },
    "review":    { "provider": "simplify" },
    "learn":     { "provider": "built-in" }
  }
}
```

```
RESEARCH: Explore agents → DISCUSS: AskUserQuestion → SPEC: /spec → PLAN: wave grouping
    ↓
VERIFY: codebase scan → IMPLEMENT: wave subagents → REVIEW: /simplify → LEARN: built-in
```

### Configuration 2: Superpowers Installed

```json
{
  "phases": {
    "research":  { "provider": "built-in" },
    "discuss":   { "provider": "superpowers" },
    "spec":      { "provider": "spec" },
    "plan":      { "provider": "built-in" },
    "verify":    { "provider": "built-in" },
    "implement": { "provider": "superpowers", "config": { "tdd": true } },
    "review":    { "provider": "superpowers" },
    "learn":     { "provider": "built-in" }
  }
}
```

```
RESEARCH: Explore agents → DISCUSS: /superpowers:brainstorm → SPEC: /spec → PLAN: wave grouping
    ↓
VERIFY: codebase scan → IMPLEMENT: TDD waves → REVIEW: dual-stage → LEARN: built-in
```

### Configuration 3: Feature-Dev Installed

```json
{
  "phases": {
    "research":  { "provider": "feature-dev" },
    "discuss":   { "provider": "built-in" },
    "spec":      { "provider": "spec" },
    "plan":      { "provider": "feature-dev" },
    "verify":    { "provider": "built-in" },
    "implement": { "provider": "built-in" },
    "review":    { "provider": "simplify" },
    "learn":     { "provider": "built-in" }
  }
}
```

```
RESEARCH: code-explorer → DISCUSS: AskUserQuestion → SPEC: /spec → PLAN: competing architects
    ↓
VERIFY: codebase scan → IMPLEMENT: wave subagents → REVIEW: /simplify → LEARN: built-in
```

### Configuration 4: Everything Installed

```json
{
  "phases": {
    "research":  { "provider": "feature-dev" },
    "discuss":   { "provider": "superpowers" },
    "spec":      { "provider": "spec" },
    "plan":      { "provider": "feature-dev" },
    "verify":    { "provider": "built-in" },
    "implement": { "provider": "superpowers", "config": { "tdd": true } },
    "review":    { "provider": "simplify+feature-dev" },
    "learn":     { "provider": "compound-engineering" }
  }
}
```

```
RESEARCH: code-explorer → DISCUSS: brainstorm → SPEC: /spec → PLAN: competing architects
    ↓
VERIFY: codebase scan → IMPLEMENT: TDD waves → REVIEW: code-reviewer + /simplify → LEARN: /ce:compound
```

---

## What We Absolutely Depend On

### Tier 1: Cannot ship without (hard blockers)

| Dependency | Reason | Risk |
|-----------|--------|------|
| **Claude Code CLI** | Runtime for everything: skills, hooks, agents, worktrees | None — this is the platform |
| **`/spec` skill** | Already built. Backbone of all spec operations. | None — already complete |
| **Phase gate hook** | Already built. Enforces phase discipline. | Low — needs updates for new phases |
| **`/develop` SKILL.md** | Already built. Needs major rewrite for config routing + new phases. | Medium — largest piece of work |

### Tier 2: Should ship with (strong defaults)

| Dependency | Reason | Risk |
|-----------|--------|------|
| **`/simplify` skill** | Default review provider. Already available. | Low — already works, just needs wiring |
| **`.framework.json` config** | Without it, no plugin routing. But built-in defaults work without it. | Low — simple JSON file |

### Tier 3: Nice to have (plugins enhance)

| Dependency | Reason | Risk |
|-----------|--------|------|
| **superpowers** | Enhances DISCUSS, IMPLEMENT, REVIEW. Not required. | None — graceful fallback |
| **feature-dev** | Enhances RESEARCH, PLAN, REVIEW. Not required. | None — graceful fallback |
| **compound-engineering** | Enhances LEARN. Not required. | None — graceful fallback |
| **`/setup-framework`** | Users can manually create `.framework.json`. Nice to automate. | None — manual workaround exists |

---

## Build Sequence

What to build and in what order, based on dependencies.

```
Wave 1 (no dependencies):
  □ Update /develop SKILL.md — add VERIFY and LEARN phases, cluster model, feature spec flow
  □ Update check-phase.sh — add VERIFY, LEARN, DISCUSS rules + new .phase format
  □ Create global plan template — multi-feature sequencing
  □ Update CLAUDE.md — reflect new architecture

Wave 2 (depends on Wave 1):
  □ Add config reader to /develop — read .framework.json, route to provider or built-in
  □ Create .framework.json default — zero-plugin config
  □ Add discussion templates — structured prompts for DISCUSS phase
  □ Update settings.json — permissions for new skills

Wave 3 (depends on Wave 2):
  □ Build /setup-framework skill — detect plugins, generate config
  □ Build detect-plugins.sh — scan for known plugin signatures
  □ Build generate-config.sh — write .framework.json from choices

Wave 4 (depends on Wave 3, only if plugins exist):
  □ Build superpowers adapter — routing logic for brainstorm, TDD, dual-review
  □ Build feature-dev adapter — routing logic for explorer, architect, reviewer
  □ Build compound-engineering adapter — routing logic for /ce:compound
```

---

## Open Questions

1. **Review chaining:** Can we chain `simplify+feature-dev` in one REVIEW phase? Or should we pick one? If chaining, what's the interface?
2. **jq dependency:** Require `jq` for JSON parsing, or build a bash-only fallback? `jq` is cleaner but adds a dependency.
3. **Plugin version detection:** How do we know if an installed plugin's interface matches what we expect? Version pins? Feature detection?
4. **VERIFY scope:** Should VERIFY also check that tests referenced in the spec still pass, or just check file/interface existence?
5. **Parallel feature implementation:** Can two features be implemented in parallel (in separate worktrees), or must they be strictly sequential?
