---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
updated: 2026-05-03
---

# shards-code — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md)
**Branches:** [product-commands.md](product-commands.md), [tech-detect-context.md](tech-detect-context.md), [tech-hooks.md](tech-hooks.md)

---

## Validation Summary

shards-code orchestrates existing systems rather than building new ones. The vast majority of capability comes from upstream:

**Already exists (don't rebuild):**
- Compound Engineering: 51 agents, 36 skills, all `/ce-*` commands
- Superpowers: 14 skills (brainstorming, TDD, subagent-driven dev, systematic debugging, etc.)
- Bundled `/spec` skill: SKILL.md, scripts (setup, validate, list-specs), templates
- Claude Code primitives: hooks, slash commands, subagent dispatch, skill loading
- Prior art in `.spec/archive/engineering-agent/insights.md`: design philosophy, hook bash patterns, structural decisions to adopt

**Must build:**
- 3 commands (`commands/code-quick.md`, `code-strategy.md`, `code-feature.md`)
- 4 hooks (`hooks/session-start.sh`, `user-prompt-submit.sh`, `pre-tool-use.sh`, `stop.sh`)
- 3 bin scripts (`bin/detect-context.sh`, `set-phase.sh`, `merge-feature.sh`)
- `install.sh`, `claude/CLAUDE.md`, `settings.json`, `.gitignore`

**Timeline:** 3 sessions for v1 (one per milestone), realistic. v1.1 adds `/code:amend` and `.shards/config.json` only after v1 has been used.

**Total LOC:** ~890 across 12 files.

---

## Critical Architecture Decisions

### Decided

- **Keystone routing.** A single `bin/detect-context.sh` returns JSON; hooks and commands read it. No second source of truth.
- **State as one line.** `.spec/.phase` holds `<workflow>:<phase>[:<feature>]`. Written only by `bin/set-phase.sh`.
- **Two hard blocks + one structural block.** PreToolUse blocks lessons.md outside COMPOUND, global specs outside SPEC/COMPOUND, and direct `.phase` edits. Everything else is advisory.
- **Three commands, no fourth.** Quick, strategy, feature. `/code:amend` deferred to v1.1.
- **Bash + jq + markdown.** No new runtime dependencies.
- **Graceful degradation.** Missing skill, corrupt `.phase`, missing `.spec/` — warnings, never crashes.
- **Symlink-based install.** `install.sh` symlinks files into `~/.claude/` and writes a per-project `<project>/.claude/settings.json`. No build step.

### To Resolve (Open)

These match the open questions in product.md and the branch docs. Defaults are listed; resolve before or during the relevant milestone.

- [ ] **OPEN-1: Quick mode plan threshold.** "Any logic change → plan, ≤2 lines literal text → direct" (default), or stricter "always plan", or looser "only multi-file/architecture plans"? Affects `commands/code-quick.md` and the heuristic in `detect-context.sh`. Default: stricter for v1; revisit after first dogfood.
- [ ] **OPEN-2: Hard-block list.** v1 has 2. Watch for footguns during real use; add only on evidence of repeated misuse.
- [ ] **OPEN-3: `.shards/config.json`.** v1 or v1.1? Default: v1.1.
- [ ] **OPEN-4: `/code:amend`.** v1 or v1.1? Default: v1.1.
- [ ] **OPEN-5: Skill auto-loading.** Stderr suggestions only (default), or actual skill activation when hooks detect a missing skill in the load list? Default: stderr-only in v1.
- [ ] **OPEN-6 (from tech-hooks.md):** Stop-hook turn-counter location: `.spec/.phase-turns` (gitignored file) vs session tmpfile. Default: gitignored file.
- [ ] **OPEN-7 (from product-commands.md):** Strategy refocus diff UX: full diff at once, or sectioned with gates. Default: sectioned.
- [ ] **OPEN-8 (from product-commands.md):** Feature reactivation from `archive/`: copy back + resume from `DESIGN:SPEC`, or treat as fresh. Default: copy + resume.
- [ ] **OPEN-9 (from tech-detect-context.md):** Per-skill availability detection (vs per-plugin). Default: per-plugin in v1.
- [ ] **OPEN-10 (from tech-hooks.md):** PreToolUse should also match `Bash` to catch `rm -rf .spec/`? Default: no in v1.

---

## Implementation Roadmap

| Milestone | Goal | Sessions | Risk |
|-----------|------|----------|------|
| **M1: Foundation** | Routing keystone + state writer + first command + visibility hook | 1 | Low |
| **M2: Enforcement** | PreToolUse phase gate + strategy command | 1 | Med |
| **M3: Lifecycle** | Feature command + merge tool + remaining hooks + installer | 1 | Med |
| **v1.1** | Amend command + provider-override config | 1 | Low (deferred) |

---

## M1: Foundation

**Goal:** A working `/code:quick` end-to-end, with the keystone routing script and state writer in place. SessionStart hook gives visibility.

**Sessions:** 1 | **Risk:** Low — small surface, clear contracts, no enforcement yet.

Tasks (build order matches the dependency chain — keystone first, things that consume it second):

- [ ] **M1.1 — `bin/detect-context.sh` (~150 LOC).** Implements the contract in [tech-detect-context.md](tech-detect-context.md). Inputs: `$1` ∈ {quick, strategy, feature, none}, optional stdin. Outputs: single JSON object on stdout. Reads `.spec/.phase` plus filesystem state. Routes per the rules table. Returns valid JSON even on bad state. Exit non-zero only when `jq` is missing.
- [ ] **M1.2 — `bin/set-phase.sh` (~30 LOC).** Validates the new phase against the grammar in [tech.md](tech.md), then writes `.spec/.phase`. Atomic write (`mv` from tmp file). Exits non-zero on invalid input with a clear stderr message. Always idempotent for the same value.
- [ ] **M1.3 — `commands/code-quick.md` (~50 LOC).** Implements the flow in [product-commands.md](product-commands.md) §1. Calls `detect-context.sh quick`, applies its `skills_to_load`, decides trivial-vs-non-trivial, optionally writes `.spec/.quick-plan.md`, executes, runs tests, light review, done.
- [ ] **M1.4 — `hooks/session-start.sh` (~40 LOC).** Per [tech-hooks.md](tech-hooks.md) §2. Stderr only. Always exits 0. Reads top 3 lessons by `### ` heading.
- [ ] **M1.5 — Wire it up.** Initial `settings.json` registering only SessionStart and the bash permissions for the keystone script. Local test on a sandbox project.

**Done when:**
- `bash bin/detect-context.sh none` returns valid JSON in all four major states (no .spec, .spec without specs, .spec with specs, with active phase).
- `bash bin/set-phase.sh quick` writes `.spec/.phase`; `bash bin/set-phase.sh garbage:state` exits non-zero.
- `/code:quick "fix typo in README"` runs end-to-end on a sandbox project and produces a single edit + git status preview.
- SessionStart prints phase and top lessons to stderr.

---

## M2: Enforcement

**Goal:** Add the gentle phase gate and the strategy command. After this milestone, `/code:strategy` produces global specs and `pre-tool-use.sh` blocks lessons.md / global specs writes outside the right phases.

**Sessions:** 1 | **Risk:** Med — getting the block conditions right matters; over-blocking is annoying, under-blocking misses the point.

Tasks:

- [ ] **M2.1 — `hooks/pre-tool-use.sh` (~100 LOC).** Per [tech-hooks.md](tech-hooks.md) §4. Reads stdin JSON, calls `detect-context.sh`, applies decision rules in order. Exit 2 with clear message on the three hard blocks. Exit 0 with stderr warning otherwise.
- [ ] **M2.2 — `commands/code-strategy.md` (~80 LOC).** Per [product-commands.md](product-commands.md) §2. Calls `set-phase.sh` to drive RESEARCH → DISCUSS → SPEC → PLAN. Delegates each phase to its skill (CE strategy/ideate, superpowers brainstorming, /spec, ce-plan). Re-runnable for refocus.
- [ ] **M2.3 — Update `settings.json`** to register PreToolUse with the `Edit|Write|NotebookEdit` matcher.
- [ ] **M2.4 — Test enforcement.** Verify: writing `.spec/lessons.md` during `quick` is blocked with a useful message. Writing `.spec/.phase` is blocked. Writing during `strategy:DESIGN:SPEC` is allowed for global specs but warns for source files.

**Done when:**
- `/code:strategy` on a fresh project produces `.spec/product.md` and `.spec/tech.md` matching the templates.
- `/code:strategy` re-runs without clobbering branch docs the user wrote by hand.
- Three hard blocks fire correctly; no false positives in normal `/code:quick` use.
- `validate.sh` passes after a strategy run.

---

## M3: Lifecycle

**Goal:** The full `/code:feature` lifecycle, merge tooling, the remaining two hooks, and the installer. This is the keystone milestone — at the end, shards-code is feature-complete for v1.

**Sessions:** 1 | **Risk:** Med — `/code:feature` is the longest command and has the most state transitions; `merge-feature.sh` has to be careful about not corrupting global specs.

Tasks:

- [ ] **M3.1 — `commands/code-feature.md` (~150 LOC).** Per [product-commands.md](product-commands.md) §3. Drives the eight phases (DESIGN:RESEARCH/DISCUSS/SPEC/PLAN, then human gate, then IMPL:VERIFY/WORK/REVIEW, then human gate, then SHIP/COMPOUND). Resumable via `.phase`.
- [ ] **M3.2 — `bin/merge-feature.sh` (~60 LOC).** Reads `.spec/features/<name>/tech.md`, identifies sections marked as cross-cutting (frontmatter `merge: true` or `--- merge ---` markers), proposes a diff to global `.spec/tech.md`, asks user to confirm, applies on yes. Then `mv .spec/features/<name>/ .spec/archive/<name>/`.
- [ ] **M3.3 — `hooks/user-prompt-submit.sh` (~50 LOC).** Per [tech-hooks.md](tech-hooks.md) §3. Keyword routing + phase-fight detection. Stderr only.
- [ ] **M3.4 — `hooks/stop.sh` (~40 LOC).** Per [tech-hooks.md](tech-hooks.md) §5. End-of-turn smell checks. Stderr only.
- [ ] **M3.5 — `claude/CLAUDE.md` (~60 LOC).** The policy doc. Decision tree, skill loading rules, spec discipline, "what I never do" list. (Content sketched in the task brief.)
- [ ] **M3.6 — `install.sh` (~80 LOC).** Symlinks: `bin/`, `hooks/`, `commands/` into the user's Claude Code config. Writes `<project>/.claude/settings.json` (offers diff if exists). Adds `.spec/.quick-plan.md` and `.spec/.phase` to `.gitignore`.
- [ ] **M3.7 — `.gitignore`** at the project root for shards-code itself.
- [ ] **M3.8 — End-to-end test.** Run `/code:feature dark-mode` on a sandbox React project: full DESIGN cluster, gate, IMPL cluster, gate, COMPOUND. Verify global tech.md gets the merged section. Verify `.spec/archive/dark-mode/` exists.

**Done when:**
- One full feature cycle (DESIGN through COMPOUND) succeeds end-to-end on a sandbox project.
- Global `.spec/tech.md` after COMPOUND shows the merged section (and only the merged section).
- All four hooks active simultaneously without latency complaints.
- `install.sh` works on a fresh machine — produces a working setup with one command.

---

## v1.1: Amend + Config

Deferred. Build only after v1 has been used on at least two real projects.

- [ ] **v1.1.1 — `commands/code-amend.md` (~60 LOC).** Per [product-commands.md](product-commands.md) §4. Mid-IMPL targeted spec amendment.
- [ ] **v1.1.2 — `.shards/config.json` schema + reader.** Allows project-level overrides for which skill handles which phase. v1 hardcodes the routing table in `detect-context.sh`; v1.1 reads from `.shards/config.json` if present, falls back to the hardcoded defaults.

**Done when:** the use cases that motivated each are demonstrated on a real project, not invented.

---

## Critical Path

```
M1 (Foundation)  →  M2 (Enforcement)  →  M3 (Lifecycle)  →  [dogfood]  →  v1.1
                                                                   ↑
                                                                   └─ deferred until needs surface
```

Each milestone is single-session-shaped and independently dogfoodable. M1 alone gives a working `/code:quick`. M1+M2 gives quick + strategy. M1+M2+M3 is v1 complete.

---

## Validation Criteria

### Per milestone
See "Done when:" under each milestone.

### v1 complete (end of M3)
- All three commands work end-to-end on a sandbox project.
- All four hooks fire at the right times with stderr output that's helpful, not noisy.
- `bin/detect-context.sh` outputs valid JSON for every (workflow, phase) combination in the grammar.
- `bin/set-phase.sh` validates and writes atomically.
- The two hard blocks fire correctly and the structural `.phase` block fires correctly.
- `install.sh` produces a working install on a fresh machine.
- `validate.sh` passes after a strategy run and after a feature COMPOUND.

### Healthy after dogfood
- One real project bootstrapped via `/code:strategy`.
- One real feature shipped via `/code:feature` end-to-end.
- At least three quick fixes via `/code:quick` without scope balloon.
- Lessons appended to `.spec/lessons.md` from at least one COMPOUND run.

---

## Progress

| Milestone | Status | Sessions Used | Estimate |
|-----------|--------|---------------|----------|
| M1: Foundation | NOT STARTED | 0 | 1 |
| M2: Enforcement | NOT STARTED | 0 | 1 |
| M3: Lifecycle | NOT STARTED | 0 | 1 |
| v1.1: Amend + Config | DEFERRED | 0 | 1 |

| Task | Milestone | Status |
|---|---|---|
| 1. `bin/detect-context.sh` | M1 | NOT STARTED |
| 2. `bin/set-phase.sh` | M1 | NOT STARTED |
| 3. `commands/code-quick.md` | M1 | NOT STARTED |
| 4. `hooks/session-start.sh` | M1 | NOT STARTED |
| 5. `hooks/pre-tool-use.sh` | M2 | NOT STARTED |
| 6. `commands/code-strategy.md` | M2 | NOT STARTED |
| 7. `commands/code-feature.md` | M3 | NOT STARTED |
| 8. `bin/merge-feature.sh` | M3 | NOT STARTED |
| 9. `hooks/user-prompt-submit.sh` | M3 | NOT STARTED |
| 10. `hooks/stop.sh` | M3 | NOT STARTED |
| 11. `claude/CLAUDE.md` | M3 | NOT STARTED |
| 12. `install.sh` | M3 | NOT STARTED |
| 13. `commands/code-amend.md` | v1.1 | DEFERRED |
| 14. `.shards/config.json` | v1.1 | DEFERRED |
