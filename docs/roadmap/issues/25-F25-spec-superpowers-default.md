---
title: "F25 [spec] spec-superpowers-default: assertive skill, router text, superpowers-first flow"
id: F25
tool: spec
phase: 3
milestone: "v0.4 Phase 3 — spec"
depends_on: []
parent: epic
---

## Deliverable
Rewrite the spec skill (`spec/SKILL.md`, `feature.md`, `strategy.md`, `reference/**`, templates) in a concise, imperative, assertive voice with a trigger-only, superpowers-style description; author the `using-spec` router text; replace the redirect/skip delegate lines in `flow/{feature,strategy}.md` so superpowers writes its native artifacts under `docs/superpowers/{specs,plans}/`; add the `superpowers-frozen` write-policy rule; un-ignore `docs/superpowers/`.

## Scope
**Owns:** skill prose and frontmatter, router text, flow delegate lines and `writes:` lists, the one policy rule, the pressure-test harness.
**Does not own:** the router hook and tooth (F19), promotion (F18), layout validators (F17).

## Acceptance (evidence, not intent)
- [ ] pressure harness: every scenario passes with superpowers present and absent, isolated `CLAUDE_CONFIG_DIR`, no CLAUDE.md
- [ ] `bash tests/run.sh` and `node flow/engine/tests/run.mjs` green
- [ ] `node .agents/skills/vibe/engine/cli.mjs render --check` green
- [ ] description ≤1024 characters, starts "You MUST use this" or "Use when"

## Depends on
—

## Decisions this implements
- D45 — superpowers-first feature layer
- D47 — router + tooth (prose half)

Phase: 3 · Tool: spec · Source: `docs/spec/plan.md` row F25
