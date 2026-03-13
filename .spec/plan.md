---
type: entrypoint
scope: implementation
covers: milestones, task breakdown, validation criteria, session planning
updated: 2026-03-13
---

# Engineering Agent — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md)

---

## Validation Summary

Already exists (don't rebuild):
- `/spec` skill — full spec management system with templates, validation, setup
- `/develop` skill — phase lifecycle with RESEARCH/SPEC/PLAN/IMPLEMENT/REVIEW
- `/simplify` skill — multi-agent code review
- Phase gate hooks — PreToolUse enforcement in `.claude/hooks/check-phase.sh`
- `.spec/.phase` tracking — phase state persistence

Must build:
- `.framework.json` config schema and reader
- Config-aware phase routing in `/develop`
- Built-in default providers for each phase
- Plugin detection scripts
- `/setup-framework` interactive installer skill
- Plugin adapter layer (superpowers, feature-dev)
- Updated CLAUDE.md reflecting the meta-framework

**Timeline:** 3-4 sessions

---

## Critical Architecture Decisions

### Decided
- **Config in `.spec/.framework.json`:** Project-specific, lives with specs (agreed)
- **Built-in defaults required:** Framework works with zero plugins (agreed)
- **Phase names unchanged:** RESEARCH, SPEC, PLAN, IMPLEMENT, REVIEW (agreed)
- **Spec system non-pluggable:** `/spec` is always the backbone (agreed)
- **Shell scripts over app code:** Bash for detection/validation, markdown for everything else (agreed)

### To Resolve
- [ ] Exact plugin adapter interface (how does /develop delegate to superpowers?)
- [ ] Per-phase model overrides in config?
- [ ] Plugin version compatibility strategy

---

## Implementation Roadmap

| Milestone | Goal | Sessions | Risk |
|-----------|------|----------|------|
| **M1** | Core config + routing | 1 | Low |
| **M2** | Built-in default providers | 1 | Low |
| **M3** | Plugin detection + installer | 1 | Med |
| **M4** | Plugin adapters + integration | 1-2 | Med |

---

## M1: Core Config + Routing

**Goal:** `/develop` reads `.framework.json` and routes phases to providers.
**Sessions:** 1 | **Risk:** Low

Tasks:
- [ ] Define `.framework.json` JSON schema (version, phases, installed_plugins)
- [ ] Add config reader to `/develop` SKILL.md — load and parse at startup
- [ ] Add fallback logic — if config missing, use all built-in defaults
- [ ] Add provider validation — warn if configured provider not installed
- [ ] Update phase transition logic to call provider-specific workflows

**Done when:** `/develop` reads config file and routes to correct provider (or built-in fallback).

---

## M2: Built-in Default Providers

**Goal:** Every phase works without any external plugins.
**Sessions:** 1 | **Risk:** Low

Tasks:
- [ ] Document built-in RESEARCH provider (parallel Explore agents)
- [ ] Document built-in DISCUSS provider (structured AskUserQuestion)
- [ ] Spec provider is already built (no change needed)
- [ ] Document built-in PLAN provider (wave-grouped plan writing)
- [ ] Document built-in IMPLEMENT provider (wave-based subagent execution)
- [ ] Document built-in REVIEW provider (self-review + spec compliance)
- [ ] Integrate GSD patterns: wave grouping, plan immutability, gap closure
- [ ] Integrate Superpowers patterns: pressure resistance, phase enforcement

**Done when:** `/develop` completes full lifecycle with `.framework.json` set to all `built-in` providers.

---

## M3: Plugin Detection + Installer

**Goal:** `/setup-framework` detects installed plugins and generates config.
**Sessions:** 1 | **Risk:** Medium

Tasks:
- [ ] Write `detect-plugins.sh` — scan `.agents/skills/` for known plugins
- [ ] Write `generate-config.sh` — create `.framework.json` from user choices
- [ ] Write `/setup-framework` SKILL.md — interactive installer flow
- [ ] Handle edge cases: partial installs, conflicting providers, re-running setup
- [ ] Validate generated config against schema

**Done when:** Running `/setup-framework` detects plugins, asks user preferences, and writes valid `.framework.json`.

---

## M4: Plugin Adapters

**Goal:** Superpowers, feature-dev, and simplify can be configured as phase providers.
**Sessions:** 1-2 | **Risk:** Medium

Tasks:
- [ ] Define adapter interface: what each plugin must expose
- [ ] Write superpowers adapter: brainstorm (DISCUSS), TDD (IMPLEMENT), review (REVIEW)
- [ ] Write feature-dev adapter: code-explorer (RESEARCH), code-architect (PLAN), code-reviewer (REVIEW)
- [ ] Write simplify adapter: multi-agent review (REVIEW) — mostly exists already
- [ ] Test each adapter with the actual plugin skills
- [ ] Document how to add new plugin adapters

**Done when:** Each supported plugin can be selected as a provider and the phase delegates correctly.

---

## Critical Path

M1 (config + routing) → M2 (built-in defaults) → M3 (installer) → M4 (plugin adapters)

M1 and M2 are partially parallelizable (routing logic + default provider docs can be written together).

---

## Progress

| Milestone | Status | Sessions Used | Estimate |
|-----------|--------|---------------|----------|
| M1 | NOT STARTED | 0 | 1 |
| M2 | NOT STARTED | 0 | 1 |
| M3 | NOT STARTED | 0 | 1 |
| M4 | NOT STARTED | 0 | 1-2 |
