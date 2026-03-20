---
type: support
scope: decisions
covers: open questions requiring user input before implementation
updated: 2026-03-14
---

# Open Questions

Decisions that need user input before we can build. Grouped by urgency.

---

## Blocks Wave 1 (answer before we start building)

### Q1: VERIFY scope — what does "spec still valid" mean?

VERIFY scans the codebase before implementing a feature. But how deep?

| Option | What It Checks | Effort |
|--------|---------------|--------|
| **A. File existence only** | Referenced file paths still exist | Low — just `stat` checks |
| **B. File + interface** | Files exist AND exported interfaces/types match spec | Medium — needs AST-level or grep-based checking |
| **C. File + interface + tests** | B + referenced tests still pass | High — runs test suite subset |

Recommendation: **B** — catches meaningful drift without the cost of running tests (IMPLEMENT will run tests anyway).

---

### Q2: LEARN phase — who merges feature specs into global?

After a feature ships, cross-cutting decisions merge into global specs. Who does this?

| Option | How |
|--------|-----|
| **A. Agent merges automatically** | LEARN phase reads feature spec, identifies cross-cutting items, updates global specs. User reviews the diff. |
| **B. Agent proposes, user merges** | LEARN phase produces a "merge proposal" (list of suggested changes to global specs). User applies manually. |
| **C. Agent merges, user confirms** | Agent makes the changes, then presents a summary. User can revert specific merges. |

Recommendation: **C** — agent does the work, user has final say. Matches the "mostly autonomous Implementation Cluster" philosophy.

---

### Q3: Feature spec isolation — how strict?

During IMPLEMENT, the agent reads the feature spec. Can it also read global specs?

| Option | Reads |
|--------|-------|
| **A. Feature spec only** | Only `.spec/features/<name>/`. Forces feature specs to be self-contained. |
| **B. Feature spec + global (read-only)** | Feature spec for the task, global specs for context (conventions, patterns). Can't write to global. |
| **C. Feature spec primary, global on-demand** | Starts with feature spec. If agent needs context not in the feature spec, it can read global specs. |

Recommendation: **B** — implementation agents need to follow project conventions (in global tech.md) but shouldn't be modifying them.

---

### Q4: `.phase` file — include feature name for Design Cluster?

The Implementation Cluster `.phase` includes the feature: `IMPL:VERIFY:dark-mode`. Should the Design Cluster also track which feature it's currently speccing?

| Option | Format |
|--------|--------|
| **A. Cluster:Phase only** | `DESIGN:SPEC` — Design Cluster works across all features, no single-feature tracking |
| **B. Cluster:Phase:Feature** | `DESIGN:SPEC:dark-mode` — tracks which feature spec is being written right now |

Recommendation: **A** — the Design Cluster works across all features simultaneously (it's a bootstrap, not per-feature). Tracking which feature is "current" during bootstrap doesn't map well to the flow.

---

## Blocks Wave 2 (answer before config routing)

### Q5: Per-phase model overrides?

Should `.framework.json` allow specifying which Claude model to use per phase?

```json
{
  "phases": {
    "research": { "provider": "built-in", "model": "haiku" },
    "implement": { "provider": "built-in", "model": "opus" }
  }
}
```

| Option | Trade-off |
|--------|-----------|
| **A. Yes, per-phase model** | More control, cheaper research phases. But adds config complexity. |
| **B. No, inherit from CLI** | Simpler. User controls model via Claude Code settings, not our config. |
| **C. Two-tier: bootstrap model + implement model** | Compromise: one model for Design Cluster, one for Implementation Cluster. |

Recommendation: **B** — keep config simple. Users can already control model in Claude Code. Adding our own model routing is over-engineering for v1.

---

### Q6: Plugin version compatibility?

How do we handle a plugin updating its interface in a breaking way?

| Option | How |
|--------|-----|
| **A. Don't — trust plugins** | If it breaks, the user re-runs `/setup-framework`. Our adapter tries and reports errors. |
| **B. Version pins in config** | `.framework.json` records plugin version. `/develop` warns if version changed. |
| **C. Feature detection** | Before delegating, check if the plugin exposes the expected skill/command. Fall back to built-in if not. |

Recommendation: **C** — feature detection is the most robust. Check if `/superpowers:brainstorm` exists before trying to invoke it. If not, fall back to built-in. No version tracking needed.

---

### Q7: Review provider chaining?

Can a phase have multiple providers that run in sequence?

```json
{
  "phases": {
    "review": { "provider": ["feature-dev", "simplify"] }
  }
}
```

| Option | How |
|--------|-----|
| **A. Single provider only** | Pick one. Simpler config, simpler routing. |
| **B. Chain support** | Array of providers, run in sequence. Output of first feeds into second. |
| **C. Primary + secondary** | One primary provider, one optional secondary. Not a general array. |

Recommendation: **A** for v1 — chaining adds complexity. Users who want both can configure `simplify` as provider and manually run `/simplify` during review. Revisit for v2.

---

## Blocks Wave 3 (answer before installer)

### Q8: Plugin detection — how do we identify plugins?

`/setup-framework` needs to know which plugins are installed. How?

| Option | How |
|--------|-----|
| **A. Known file paths** | Check for `.agents/skills/superpowers/SKILL.md`, etc. Hardcoded list of known plugins. |
| **B. Manifest file** | Plugins declare themselves via a `plugin.json` with name, version, provided phases. |
| **C. Skill name convention** | Scan `.agents/skills/*/SKILL.md` and grep for phase provider declarations. |

Recommendation: **A** — we support a small, curated set of plugins (not a marketplace). Hardcoded detection for 3-4 known plugins is simpler and more reliable than any discovery protocol.

---

### Q9: What happens when a plugin is uninstalled?

User had superpowers providing DISCUSS, then removes the plugin.

| Option | Behavior |
|--------|----------|
| **A. Silent fallback** | `/develop` notices provider missing, falls back to built-in, continues. |
| **B. Warn and fallback** | Shows warning ("superpowers not found, using built-in for DISCUSS"), then continues. |
| **C. Block and re-configure** | Refuses to proceed. Forces user to re-run `/setup-framework`. |

Recommendation: **B** — warn but don't block. The framework should always be able to proceed.

---

## Non-Blocking (decide anytime, good to know before Wave 4)

### Q10: Should we support custom built-in overrides?

Can users customize the built-in defaults without installing a plugin? E.g., "I want the built-in RESEARCH but with 6 agents instead of 4."

| Option | How |
|--------|-----|
| **A. No customization** | Built-in defaults are fixed. Want different? Install a plugin. |
| **B. Config options per phase** | `"research": { "provider": "built-in", "config": { "agents": 6 } }` |
| **C. Prompt overrides** | `"research": { "provider": "built-in", "prompt_override": ".agents/skills/develop/prompts/my-research.md" }` |

Recommendation: **A** for v1. Built-in defaults should be opinionated and good enough. Customization is a v2 concern.

---

### Q11: Merge conflict strategy for global specs?

Feature A ships and merges into global tech.md. Feature B (designed before A shipped) has a spec that now conflicts with the updated global tech.md. What happens during Feature B's VERIFY?

| Option | Behavior |
|--------|----------|
| **A. Flag and pause** | VERIFY detects the conflict, flags it to the user, pauses until resolved. |
| **B. Auto-amend feature spec** | VERIFY detects the conflict, amends Feature B's spec to align with new global reality. |
| **C. Flag with suggestion** | VERIFY detects, proposes an amendment, asks user to confirm. |

Recommendation: **C** — the agent should do the analysis work but not silently change specs. Present the conflict and proposed resolution, user confirms.

---

### Q12: Archive retention?

How long do archived feature specs (`.spec/archive/<name>/`) stick around?

| Option | How |
|--------|-----|
| **A. Forever** | Git tracks them. They're small. Never delete. |
| **B. Configurable TTL** | Config specifies how many archived features to keep. Oldest get deleted. |
| **C. Manual cleanup** | User deletes when they want. No automation. |

Recommendation: **A** — they're small markdown files in git. Storage is free. Historical context is valuable. Don't over-engineer this.

---

## From Real-World Deployment (blocks Wave 1, task 1.6)

### Q13: Should `context.md` be created during `/spec setup` by default?

The deployment showed `context.md` (business/domain context) is essential for rework projects but unnecessary for greenfield.

| Option | Behavior |
|--------|----------|
| **A. Always create** | `/spec setup` always creates `context.md` template. User fills it or deletes it. |
| **B. Ask during setup** | `/spec setup` asks "Is this an existing codebase?" If yes, creates `context.md` + research/ + docs/. |
| **C. Never auto-create** | User manually creates if needed. Template available but not scaffolded. |

Recommendation: **B** — the setup flow should detect greenfield vs rework and scaffold accordingly.

---

### Q14: Should `docs/` and `reference/` be validated by `/spec validate`?

These directories contain reference material, not specs. Should the validation script check them?

| Option | Behavior |
|--------|----------|
| **A. Skip entirely** | Validation ignores docs/ and reference/. They're unstructured support material. |
| **B. Check existence only** | Validation confirms directories exist if referenced by specs, but doesn't validate content. |
| **C. Light validation** | Check that referenced files exist (e.g., if product-design.md links to reference/mockup.png, verify the file is there). |

Recommendation: **C** — broken references are the main risk. Validate links, not content.

---

### Q15: How do `design` scope docs interact with the product/tech separation?

Design docs cross the product/tech line. How do we handle this in practice?

| Option | Naming |
|--------|--------|
| **A. `product-design-*.md`** | Design docs are product-prefixed but allowed to contain tech. The prefix indicates they're primarily product-facing. |
| **B. `design-*.md`** | New prefix. Not product, not tech — its own thing. |
| **C. Either prefix works** | `product-design-*.md` or `tech-design-*.md` depending on emphasis. |

Recommendation: **A** — design docs are primarily product-facing (they describe what the user sees). The `product-design-` prefix maintains the naming convention while the `design` scope flag relaxes the content rules.

---

## Summary

| Question | Blocks | Recommendation | Status |
|----------|--------|---------------|--------|
| Q1: VERIFY scope | Wave 1 | B (file + interface) | OPEN |
| Q2: LEARN merge strategy | Wave 1 | C (agent merges, user confirms) | OPEN |
| Q3: Feature spec isolation | Wave 1 | B (feature + global read-only) | OPEN |
| Q4: .phase format for Design | Wave 1 | A (cluster:phase only) | OPEN |
| Q5: Per-phase model overrides | Wave 2 | B (no, inherit from CLI) | OPEN |
| Q6: Plugin version compat | Wave 2 | C (feature detection) | OPEN |
| Q7: Review chaining | Wave 2 | A (single provider for v1) | OPEN |
| Q8: Plugin detection method | Wave 3 | A (known file paths) | OPEN |
| Q9: Plugin uninstall behavior | Wave 3 | B (warn and fallback) | OPEN |
| Q10: Custom built-in overrides | Non-blocking | A (no for v1) | OPEN |
| Q11: Merge conflict strategy | Non-blocking | C (flag with suggestion) | OPEN |
| Q12: Archive retention | Non-blocking | A (forever) | OPEN |
| Q13: context.md in setup | Wave 1 (1.6) | B (ask greenfield vs rework) | OPEN |
| Q14: docs/reference/ validation | Wave 1 (1.6) | C (validate links only) | OPEN |
| Q15: Design scope naming | Wave 1 (1.6) | A (product-design-*.md prefix) | OPEN |
