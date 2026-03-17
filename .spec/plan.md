---
type: entrypoint
scope: implementation
covers: milestones, task breakdown, build waves, current state, progress tracking
updated: 2026-03-14
---

# Engineering Agent — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md)
**Integration map:** [product-integration.md](product-integration.md)
**Open questions:** [questions.md](questions.md)

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
| Global specs | **Complete** | `.spec/product.md`, `tech.md`, `product-design.md`, `product-integration.md` |
| Research docs | **Complete** | `.spec/research/` — 7 phase analyses + agreements + learn phase study |
| Lessons | **Started** | `.spec/lessons.md` — 1 lesson captured |

### What's Missing

| Component | Priority | Effort | Blocked By |
|-----------|----------|--------|-----------|
| `/develop` SKILL.md rewrite (clusters + 8 phases) | P0 | Large | Nothing — can start now |
| Hook updates (new phases + `.phase` format) | P0 | Small | Nothing — can start now |
| VERIFY phase logic | P0 | Medium | `/develop` rewrite |
| LEARN phase logic | P0 | Medium | `/develop` rewrite |
| Feature spec directory conventions | P0 | Small | `/develop` rewrite |
| `.framework.json` schema + config reader | P0 | Medium | Nothing — can start now |
| Global plan template (multi-feature) | P0 | Small | Nothing — can start now |
| CLAUDE.md + Agents.md updates | P0 | Small | All P0 tasks |
| `/setup-framework` skill | P1 | Medium | `.framework.json` schema |
| Discussion prompt templates | P1 | Small | `/develop` rewrite |
| Plugin adapters (superpowers, feature-dev, compound-eng) | P2 | Medium each | `/setup-framework` |

---

## Build Waves

### Wave 1: Foundation (no dependencies, all parallel)

Everything in this wave can be built simultaneously — no task depends on another.

**1.1 — Rewrite `/develop` SKILL.md**
The largest piece of work. Rewrite the skill to reflect:
- Two clusters: Design Cluster (bootstrap) and Implementation Cluster (per-feature)
- 8 phases: RESEARCH, DISCUSS, SPEC, PLAN, VERIFY, IMPLEMENT, REVIEW, LEARN
- Feature spec directory flow: create during SPEC, consume during VERIFY+IMPLEMENT, merge during LEARN, archive after
- Global plan as the feature sequencer
- Config-driven routing stub (reads `.framework.json` or falls back to built-in)
- Session resumption with `CLUSTER:PHASE:FEATURE` format
- Per-phase built-in defaults written out explicitly

Files touched:
- `.agents/skills/develop/SKILL.md` (full rewrite)
- `.agents/skills/develop/README.md` (update)

**1.2 — Update phase gate hook**
Update `check-phase.sh` to support:
- New phases: DISCUSS, VERIFY, LEARN
- New `.phase` format: `DESIGN:RESEARCH`, `IMPL:VERIFY:dark-mode`, etc.
- Feature spec directory write rules (`.spec/features/<name>/`)
- LEARN phase: allow writes to `.spec/*.md`, `lessons.md`, `.spec/archive/`

Files touched:
- `.claude/hooks/check-phase.sh`

**1.3 — Define `.framework.json` schema**
Create the JSON schema and a default config (zero plugins).

Files touched:
- `.agents/skills/develop/schema/framework.schema.json` (new)
- `.agents/skills/develop/defaults/framework.json` (new — zero-plugin default)

**1.4 — Create global plan template**
New template for multi-feature plans that sequence features with cross-dependencies.

Files touched:
- `.agents/skills/spec/reference/templates/plan-global.md` (new)

**1.5 — Create feature spec templates**
Templates for feature-level product.md and tech.md inside `.spec/features/<name>/`.

Files touched:
- `.agents/skills/spec/reference/templates/feature-product.md` (new)
- `.agents/skills/spec/reference/templates/feature-tech.md` (new)

**Done when:** `/develop` can be invoked, reads the new `.phase` format, understands both clusters, and has built-in defaults for all 8 phases. Hook enforces phase-appropriate writes for all phases.

---

### Wave 2: Integration (depends on Wave 1)

**2.1 — Add config reader to `/develop`**
The routing logic that reads `.framework.json` and dispatches to the correct provider. If config is missing or provider is `"built-in"`, use the built-in default. If provider is a plugin name, delegate to that skill.

Files touched:
- `.agents/skills/develop/SKILL.md` (add routing section)
- `.agents/skills/develop/scripts/read-config.sh` (new — parse `.framework.json`)

**2.2 — Create discussion prompt templates**
Structured question sets for the DISCUSS phase, organized by feature type (UI, API, data, infrastructure). The built-in DISCUSS provider uses these with AskUserQuestion.

Files touched:
- `.agents/skills/develop/prompts/discuss-ui.md` (new)
- `.agents/skills/develop/prompts/discuss-api.md` (new)
- `.agents/skills/develop/prompts/discuss-data.md` (new)
- `.agents/skills/develop/prompts/discuss-general.md` (new)

**2.3 — Update settings.json**
Add permissions for new skills and update hook matchers.

Files touched:
- `.claude/settings.json`

**2.4 — Update CLAUDE.md and Agents.md**
Reflect the new architecture: two clusters, 8 phases, feature specs, config routing.

Files touched:
- `CLAUDE.md`
- `Agents.md`

**Done when:** `/develop` reads config, routes to providers (or built-in), DISCUSS has structured prompts, and all documentation reflects the new architecture.

---

### Wave 3: Installer (depends on Wave 2)

**3.1 — Build `/setup-framework` skill**
Interactive plugin detection and config generation. Scans `.agents/skills/` for known plugin signatures, asks the user which provider to use per phase, writes `.framework.json`.

Files touched:
- `.agents/skills/setup-framework/SKILL.md` (new)
- `.agents/skills/setup-framework/README.md` (new)
- `.agents/skills/setup-framework/scripts/detect-plugins.sh` (new)
- `.agents/skills/setup-framework/scripts/generate-config.sh` (new)

**Done when:** `/setup-framework` detects installed plugins, presents choices, generates valid `.framework.json`.

---

### Wave 4: Plugin Adapters (depends on Wave 3, only if plugins exist)

**4.1 — Superpowers adapter**
Routing logic for: DISCUSS → `/superpowers:brainstorm`, IMPLEMENT → TDD enforcement, REVIEW → dual-stage review.

**4.2 — Feature-dev adapter**
Routing logic for: RESEARCH → code-explorer agents, PLAN → competing code-architect agents, REVIEW → code-reviewer with confidence.

**4.3 — Compound-engineering adapter**
Routing logic for: LEARN → `/ce:compound`.

All adapters are sections within `/develop` SKILL.md — conditional routing based on provider name.

**Done when:** Each plugin can be selected in `.framework.json` and the phase delegates correctly.

---

## Critical Path

```
Wave 1 ──────────────────────────────────────────── Wave 2 ─────────── Wave 3 ─── Wave 4
  1.1 /develop rewrite ──┐                           2.1 config reader
  1.2 hook updates ──────┤                           2.2 discuss prompts
  1.3 .framework.json ───┼── all parallel ──────────▶ 2.3 settings.json  ──▶ 3.1 /setup-framework ──▶ 4.x adapters
  1.4 global plan tpl ───┤                           2.4 CLAUDE.md update
  1.5 feature spec tpl ──┘
```

Wave 1 is the bulk of the work. Waves 2-4 are incremental.

---

## Validation Criteria

### Minimum Viable Framework (end of Wave 2)

- [ ] `/develop` orchestrates 8 phases across 2 clusters
- [ ] Design Cluster: RESEARCH → DISCUSS → SPEC → PLAN (bootstrap, writes global + feature specs)
- [ ] Implementation Cluster: VERIFY → IMPLEMENT → REVIEW → LEARN (per-feature, repeating)
- [ ] Phase gate hook enforces write rules for all 8 phases
- [ ] `.phase` file uses `CLUSTER:PHASE:FEATURE` format
- [ ] Feature specs created in `.spec/features/<name>/`, archived after LEARN
- [ ] Built-in defaults work for every phase with zero plugins
- [ ] Config reader loads `.framework.json` or falls back to defaults
- [ ] CLAUDE.md and Agents.md reflect current architecture

### Full Framework (end of Wave 4)

- [ ] `/setup-framework` detects plugins and generates config
- [ ] Superpowers, feature-dev, compound-engineering can be selected as providers
- [ ] Each provider delegates correctly to the plugin's skill

---

## Progress

| Wave | Task | Status | Notes |
|------|------|--------|-------|
| 1 | 1.1 `/develop` SKILL.md rewrite | NOT STARTED | Largest task |
| 1 | 1.2 Hook updates | NOT STARTED | |
| 1 | 1.3 `.framework.json` schema | NOT STARTED | |
| 1 | 1.4 Global plan template | NOT STARTED | |
| 1 | 1.5 Feature spec templates | NOT STARTED | |
| 2 | 2.1 Config reader | NOT STARTED | |
| 2 | 2.2 Discussion prompts | NOT STARTED | |
| 2 | 2.3 Settings.json update | NOT STARTED | |
| 2 | 2.4 CLAUDE.md + Agents.md | NOT STARTED | |
| 3 | 3.1 `/setup-framework` skill | NOT STARTED | |
| 4 | 4.1 Superpowers adapter | NOT STARTED | |
| 4 | 4.2 Feature-dev adapter | NOT STARTED | |
| 4 | 4.3 Compound-eng adapter | NOT STARTED | |
