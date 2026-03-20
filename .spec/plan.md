---
type: entrypoint
scope: implementation
covers: milestones, task breakdown, build waves, current state, progress tracking
updated: 2026-03-19
---

# Engineering Agent — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md)
**Integration map:** [product-integration.md](product-integration.md)
**Functional design:** [product-design.md](product-design.md)

---

## Current State

### What Exists

| Component | Status | Location |
|-----------|--------|----------|
| `/spec` skill | **Complete** | `.agents/skills/spec/` — templates, validation, setup, all working |
| `/develop` skill | **Partial** | `.agents/skills/develop/` — 5-phase lifecycle. Needs rewrite for clusters, VERIFY, LEARN, config routing |
| `/simplify` skill | **Available** | Referenced, available as skill. Default REVIEW provider |
| Phase gate hook | **Partial** | `.claude/hooks/check-phase.sh` — works for 5 phases. Needs VERIFY, LEARN, DISCUSS, new `.phase` format |
| Session start hook | **Complete** | `.claude/settings.json` — shows phase + lessons reminder |
| Global specs | **Draft** | `.spec/product.md`, `tech.md`, `product-design.md`, `product-integration.md` — exist but need revision |
| Research docs | **Complete** | `.spec/research/` — 7 phase analyses + agreements + learn phase study + platform reference |
| Lessons | **Started** | `.spec/lessons.md` — 4 lessons captured |

### What's Missing

The specs describe the *vision* (two clusters, 8 phases, feature specs, plugin routing) but several areas need deeper product and technical specification before implementation can begin:

| Gap | Type | Why It Matters |
|-----|------|---------------|
| **Product spec needs platform grounding** | Product | Research uncovered Claude Code capabilities (hooks, subagents, skills, MCP, plugins) that should shape the product design. Current product.md was written before platform research. |
| **Tech spec is skeletal** | Tech | `tech.md` has architecture overview but no tech branch docs. No `tech-develop-skill.md`, no `tech-hooks.md`, no `tech-config-system.md`. Implementation will drift without detailed tech specs. |
| **Spec skill extensions unspecced** | Product + Tech | New file types (context.md, design scope, docs/, reference/) described in product.md but not formally specced in a tech branch doc. |
| **VERIFY and LEARN phases underspecced** | Product + Tech | product-design.md has flowcharts but tech details (what scripts, what subagent config, what file operations) are missing. |
| **Plugin adapter interfaces unspecced** | Tech | product-integration.md lists routing tables but the actual interface contract (input/output, error handling, fallback chain) needs a tech spec. |
| **No feature-level specs exist** | Both | The framework's own features (develop skill, hook system, config system, installer) should have `.spec/features/` directories — eating our own dogfood. |

---

## Milestones

### Milestone 0: Product & Tech Specs (CURRENT)

**Goal:** Finalize the product and technical specifications so implementation has a solid foundation. This is the Design Cluster for our own framework.

**Why first:** Everything downstream — the develop skill rewrite, hook updates, config schema, templates — depends on clear specs. Writing code against draft specs creates rework.

#### 0.1 — Revise `product.md`

Ground the product spec in platform capabilities discovered during research. The current product.md describes the vision correctly but was written before the Claude Code platform research. Update to reflect:

- How Claude Code's skill system maps to our skill design (frontmatter options, tool restrictions, context forking)
- How hooks actually work (all 20+ event types, not just PreToolUse) and what that enables beyond phase gates
- How subagents work (built-in types, custom agent definitions, worktree isolation, memory) and what that means for our wave execution model
- How the plugin system works (plugin.json manifest, skill namespacing, bundled MCP/LSP) and what that means for our plugin adapters
- Resolve open questions in product.md using platform knowledge

Files:
- `.spec/product.md` (revise)

#### 0.2 — Revise `product-design.md`

Update functional design to reflect platform-grounded decisions:

- Session resumption: how does it interact with Claude Code's auto-memory and SessionStart hooks?
- Phase gates: what hook events beyond PreToolUse should we use? (Stop hooks for completion validation, SubagentStart for context injection, PostToolUse for progress tracking)
- Unplanned features: how does the mini Design Cluster interact with worktree isolation?

Files:
- `.spec/product-design.md` (revise)

#### 0.3 — Revise `product-integration.md`

Update integration map now that we know the actual plugin system:

- Plugin adapters should use Claude Code's native plugin structure (`.claude-plugin/plugin.json`), not custom routing
- Review chaining: can we use Claude Code's subagent composition instead of custom chaining logic?
- Provider detection: use plugin manifest inspection instead of directory scanning

Files:
- `.spec/product-integration.md` (revise)

#### 0.4 — Write `tech.md` (full rewrite)

The current tech.md is skeletal. Rewrite with:

- Full architecture showing how our skills, hooks, agents, and config map to Claude Code primitives
- Tech stack: what Claude Code features we use, what shell scripts we write, what's pure markdown instruction
- State management: `.phase` file format, `.framework.json` lifecycle, spec directory conventions
- Subagent strategy: which built-in agents we use, which custom agents we define, model selection per task
- Hook architecture: complete hook chain (SessionStart → PreToolUse → PostToolUse → Stop)
- Error handling: what happens when plugins are missing, config is invalid, phases fail

Files:
- `.spec/tech.md` (full rewrite)

#### 0.5 — Write tech branch docs

Create detailed tech specs for each major subsystem:

- `tech-develop-skill.md` — The `/develop` SKILL.md: frontmatter, phase orchestration logic, config reading, provider routing, session resumption, built-in defaults for all 8 phases
- `tech-hooks.md` — Hook system: `check-phase.sh` logic, phase write rules, `.phase` file format, SessionStart context injection, Stop hook for completion gates
- `tech-config-system.md` — `.framework.json` schema, validation, defaults, config reading (jq vs bash), plugin detection
- `tech-spec-extensions.md` — New spec file types: context.md, design scope, docs/, reference/, feature spec directories, archive flow

Files:
- `.spec/tech-develop-skill.md` (new)
- `.spec/tech-hooks.md` (new)
- `.spec/tech-config-system.md` (new)
- `.spec/tech-spec-extensions.md` (new)

#### 0.6 — Update `lessons.md`

Capture any new lessons from the research and spec revision process.

Files:
- `.spec/lessons.md` (update)

**Done when:** All product specs reflect platform reality. All tech subsystems have dedicated branch docs. No implementation question requires guessing — the specs answer it.

**Exit criteria:**
- [ ] product.md grounded in Claude Code platform capabilities
- [ ] product-design.md updated with platform-aware interaction patterns
- [ ] product-integration.md updated with native plugin system
- [ ] tech.md fully rewritten with complete architecture
- [ ] 4 tech branch docs written (develop-skill, hooks, config-system, spec-extensions)
- [ ] All specs pass validation (`bash ~/.agents/skills/spec/scripts/validate.sh`)
- [ ] Open questions in product.md resolved or explicitly deferred with rationale
- [ ] lessons.md updated

---

### Milestone 1: Foundation (depends on Milestone 0)

Everything in this milestone can be built in parallel — no task depends on another. Implementation follows the tech specs written in Milestone 0.

#### 1.1 — Rewrite `/develop` SKILL.md

Per `tech-develop-skill.md`. The largest piece of work:

- Two clusters: Design Cluster (bootstrap) and Implementation Cluster (per-feature)
- 8 phases: RESEARCH, DISCUSS, SPEC, PLAN, VERIFY, IMPLEMENT, REVIEW, LEARN
- Feature spec directory flow: create during SPEC, consume during VERIFY+IMPLEMENT, merge during LEARN, archive after
- Global plan as the feature sequencer
- Config-driven routing stub (reads `.framework.json` or falls back to built-in)
- Session resumption with `CLUSTER:PHASE:FEATURE` format
- Per-phase built-in defaults written out explicitly

Files:
- `.agents/skills/develop/SKILL.md` (full rewrite)

#### 1.2 — Update phase gate hook

Per `tech-hooks.md`:

- New phases: DISCUSS, VERIFY, LEARN
- New `.phase` format: `DESIGN:RESEARCH`, `IMPL:VERIFY:dark-mode`, etc.
- Feature spec directory write rules (`.spec/features/<name>/`)
- LEARN phase: allow writes to `.spec/*.md`, `lessons.md`, `.spec/archive/`

Files:
- `.claude/hooks/check-phase.sh`

#### 1.3 — Define `.framework.json` schema

Per `tech-config-system.md`:

- JSON schema for validation
- Default config (zero plugins)
- Config reading approach (jq vs bash decision from tech spec)

Files:
- `.agents/skills/develop/schema/framework.schema.json` (new)
- `.agents/skills/develop/defaults/framework.json` (new)

#### 1.4 — Create global plan template

Multi-feature plan template that sequences features with cross-dependencies.

Files:
- `.agents/skills/spec/reference/templates/plan-global.md` (new)

#### 1.5 — Create feature spec templates

Templates for feature-level product.md and tech.md inside `.spec/features/<name>/`.

Files:
- `.agents/skills/spec/reference/templates/feature-product.md` (new)
- `.agents/skills/spec/reference/templates/feature-tech.md` (new)

#### 1.6 — Add new spec file types to `/spec` skill

Per `tech-spec-extensions.md`:

- `context.md` — optional entrypoint for business/domain context
- `docs/` — reference documentation directory
- `reference/` — visual assets directory
- `design` scope — branch docs that cross product/tech line
- "Current State" section in product.md template for rework projects

Files:
- `.agents/skills/spec/SKILL.md` (update)
- `.agents/skills/spec/reference/templates/context.md` (new)
- `.agents/skills/spec/reference/templates/product.md` (update)
- `.agents/skills/spec/reference/templates/product-design-xxx.md` (new)
- `.agents/skills/spec/scripts/validate.sh` (update)
- `.agents/skills/spec/scripts/setup.sh` (update)

**Done when:** `/develop` can be invoked, reads the new `.phase` format, understands both clusters, and has built-in defaults for all 8 phases. Hook enforces phase-appropriate writes for all phases. `/spec` recognizes all file types.

---

### Milestone 2: Integration (depends on Milestone 1)

#### 2.1 — Add config reader to `/develop`

Routing logic that reads `.framework.json` and dispatches to the correct provider.

Files:
- `.agents/skills/develop/SKILL.md` (add routing section)
- `.agents/skills/develop/scripts/read-config.sh` (new)

#### 2.2 — Create discussion prompt templates

Structured question sets for the DISCUSS phase, organized by feature type.

Files:
- `.agents/skills/develop/prompts/discuss-ui.md` (new)
- `.agents/skills/develop/prompts/discuss-api.md` (new)
- `.agents/skills/develop/prompts/discuss-data.md` (new)
- `.agents/skills/develop/prompts/discuss-general.md` (new)

#### 2.3 — Update settings.json

Add permissions for new skills and update hook matchers.

Files:
- `.claude/settings.json`

#### 2.4 — Update CLAUDE.md

Reflect the new architecture: two clusters, 8 phases, feature specs, config routing.

Files:
- `CLAUDE.md`

**Done when:** `/develop` reads config, routes to providers (or built-in), DISCUSS has structured prompts, and documentation reflects the new architecture.

---

### Milestone 3: Installer (depends on Milestone 2)

#### 3.1 — Build `/setup-framework` skill

Interactive plugin detection and config generation.

Files:
- `.agents/skills/setup-framework/SKILL.md` (new)
- `.agents/skills/setup-framework/scripts/detect-plugins.sh` (new)
- `.agents/skills/setup-framework/scripts/generate-config.sh` (new)

**Done when:** `/setup-framework` detects installed plugins, presents choices, generates valid `.framework.json`.

---

### Milestone 4: Plugin Adapters (depends on Milestone 3, only if plugins exist)

#### 4.1 — Superpowers adapter

DISCUSS → `/superpowers:brainstorm`, IMPLEMENT → TDD enforcement, REVIEW → dual-stage review.

#### 4.2 — Feature-dev adapter

RESEARCH → code-explorer agents, PLAN → competing code-architect agents, REVIEW → code-reviewer with confidence.

#### 4.3 — Compound-engineering adapter

LEARN → `/ce:compound`.

All adapters are sections within `/develop` SKILL.md — conditional routing based on provider name.

**Done when:** Each plugin can be selected in `.framework.json` and the phase delegates correctly.

---

## Critical Path

```
Milestone 0 (specs) ──────────────── Milestone 1 ─────────── Milestone 2 ─── Milestone 3 ─── Milestone 4
  0.1 product.md revision ──┐         1.1 /develop rewrite
  0.2 product-design.md ────┤         1.2 hook updates
  0.3 product-integration ──┤         1.3 .framework.json        2.1 config     3.1 /setup-     4.x adapters
  0.4 tech.md rewrite ──────┼─────▶   1.4 global plan tpl  ──▶  2.2 prompts ──▶   framework ──▶
  0.5 tech branch docs ─────┤         1.5 feature spec tpl      2.3 settings
  0.6 lessons.md ───────────┘         1.6 /spec extensions      2.4 CLAUDE.md
```

Milestone 0 is the current blocker. Without finalized specs, implementation will drift.

---

## Validation Criteria

### Design Complete (end of Milestone 0)

- [ ] All product specs grounded in Claude Code platform capabilities
- [ ] All tech subsystems have dedicated branch docs with implementation detail
- [ ] No open questions that block implementation
- [ ] Specs pass validation

### Minimum Viable Framework (end of Milestone 2)

- [ ] `/develop` orchestrates 8 phases across 2 clusters
- [ ] Design Cluster: RESEARCH → DISCUSS → SPEC → PLAN (bootstrap, writes global + feature specs)
- [ ] Implementation Cluster: VERIFY → IMPLEMENT → REVIEW → LEARN (per-feature, repeating)
- [ ] Phase gate hook enforces write rules for all 8 phases
- [ ] `.phase` file uses `CLUSTER:PHASE:FEATURE` format
- [ ] Feature specs created in `.spec/features/<name>/`, archived after LEARN
- [ ] Built-in defaults work for every phase with zero plugins
- [ ] Config reader loads `.framework.json` or falls back to defaults
- [ ] CLAUDE.md reflects current architecture

### Full Framework (end of Milestone 4)

- [ ] `/setup-framework` detects plugins and generates config
- [ ] Superpowers, feature-dev, compound-engineering can be selected as providers
- [ ] Each provider delegates correctly to the plugin's skill

---

## Progress

| Milestone | Task | Status | Notes |
|-----------|------|--------|-------|
| 0 | 0.1 Revise `product.md` | NOT STARTED | Ground in platform research |
| 0 | 0.2 Revise `product-design.md` | NOT STARTED | Platform-aware interaction patterns |
| 0 | 0.3 Revise `product-integration.md` | NOT STARTED | Native plugin system |
| 0 | 0.4 Rewrite `tech.md` | NOT STARTED | Currently skeletal |
| 0 | 0.5 Write tech branch docs (4 docs) | NOT STARTED | develop-skill, hooks, config, spec-extensions |
| 0 | 0.6 Update `lessons.md` | NOT STARTED | |
| 1 | 1.1 `/develop` SKILL.md rewrite | NOT STARTED | Largest implementation task |
| 1 | 1.2 Hook updates | NOT STARTED | |
| 1 | 1.3 `.framework.json` schema | NOT STARTED | |
| 1 | 1.4 Global plan template | NOT STARTED | |
| 1 | 1.5 Feature spec templates | NOT STARTED | |
| 1 | 1.6 New spec file types (`/spec` skill) | NOT STARTED | context.md, docs/, reference/, design scope |
| 2 | 2.1 Config reader | NOT STARTED | |
| 2 | 2.2 Discussion prompts | NOT STARTED | |
| 2 | 2.3 Settings.json update | NOT STARTED | |
| 2 | 2.4 CLAUDE.md update | NOT STARTED | |
| 3 | 3.1 `/setup-framework` skill | NOT STARTED | |
| 4 | 4.1 Superpowers adapter | NOT STARTED | |
| 4 | 4.2 Feature-dev adapter | NOT STARTED | |
| 4 | 4.3 Compound-eng adapter | NOT STARTED | |
