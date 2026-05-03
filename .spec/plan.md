---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
updated: 2026-05-03
---

# shards-code — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md)
**Features:** [features/commands/](features/commands/product.md), [features/routing/](features/routing/product.md), [features/hooks/](features/hooks/product.md)

---

## Validation Summary

shards-code orchestrates existing systems rather than building new ones. Most capability comes from upstream:

**Already exists (don't rebuild):**
- Compound Engineering: 51 agents, 36 skills, all `/ce-*` commands
- Superpowers: 14 skills (brainstorming, TDD, subagent-driven dev, systematic debugging, etc.)
- Bundled `/spec` skill: SKILL.md, scripts (setup, validate, list-specs), templates
- Claude Code primitives: hooks, slash commands, subagent dispatch, skill loading
- Prior art in `archive/engineering-agent/insights.md`: design philosophy, hook bash patterns, structural decisions

**Must build:** ~890 LOC across 12 files, decomposed into three features:

| Feature | Files | LOC est |
|---|---|---|
| **routing** | `bin/detect-context.sh`, `bin/set-phase.sh` | ~180 |
| **commands** | `commands/code-{quick,strategy,feature,amend}.md`, `bin/merge-feature.sh` | ~400 |
| **hooks** | `hooks/{session-start,user-prompt-submit,pre-tool-use,stop}.sh`, `settings.json` | ~260 |
| (basic implementation) | `claude/CLAUDE.md`, `install.sh`, `.gitignore`, `README.md` | ~150 |

**Timeline:** 3 sessions for v1 (one per milestone). v1.1 adds `/code:amend` and `.shards/config.json` only after v1 has been used on real projects.

---

## Critical Architecture Decisions

### Decided

- **Keystone routing.** A single `bin/detect-context.sh` returns JSON; hooks and commands read it. No second source of truth.
- **State as one line.** `.spec/.phase` holds `<workflow>:<phase>[:<feature>]`. Written only by `bin/set-phase.sh`.
- **Three hard blocks.** PreToolUse blocks lessons.md outside COMPOUND, global specs outside SPEC/COMPOUND, and direct `.phase` edits. Everything else advisory.
- **Three commands.** Quick, strategy, feature. `/code:amend` deferred to v1.1.
- **Bash + jq + markdown.** No new runtime dependencies.
- **Graceful degradation.** Missing skill, corrupt `.phase`, missing `.spec/` → warnings, never crashes.
- **Symlink-based install.** No build step.

### To Resolve (Open)

- [ ] **OPEN-1: Quick mode plan threshold.** Default: stricter for v1, revisit after first dogfood. Affects [features/commands/](features/commands/product.md).
- [ ] **OPEN-2: Hard-block list.** v1 has 3 (lessons, global specs, .phase). Watch for footguns; add only on evidence. Affects [features/hooks/](features/hooks/product.md).
- [ ] **OPEN-3: `.shards/config.json`.** v1 or v1.1? Default: v1.1.
- [ ] **OPEN-4: `/code:amend`.** v1 or v1.1? Default: v1.1.
- [ ] **OPEN-5: Skill auto-loading.** Stderr-only (default) or actual skill activation? Default: stderr-only in v1.
- [ ] **OPEN-6: Stop-hook turn counter location.** `.spec/.phase-turns` (default) vs session tmpfile.
- [ ] **OPEN-7: Strategy refocus diff UX.** Sectioned (default) vs full-diff.
- [ ] **OPEN-8: Feature reactivation from archive/.** Copy + resume from SPEC (default) vs fresh DESIGN.
- [ ] **OPEN-9: Per-skill availability detection.** v1 = per-plugin. Per-skill is v1.1.
- [ ] **OPEN-10: PreToolUse Bash matcher.** v1 = no. Add only on evidence of misuse.

---

## Implementation Roadmap

| Milestone | Goal | Sessions | Risk |
|-----------|------|----------|------|
| **M1: Foundation** | Routing keystone + state writer + first command + visibility hook | 1 | Low |
| **M2: Enforcement** | PreToolUse phase gate + strategy command | 1 | Med |
| **M3: Lifecycle** | Feature command + merge tooling + remaining hooks + installer | 1 | Med |
| **v1.1** | Amend command + provider-override config | 1 | Low (deferred) |

---

## M1: Foundation

**Goal:** Working `/code:quick` end-to-end. Keystone in place. SessionStart visibility hook firing.

**Sessions:** 1 | **Risk:** Low.

| Task | Feature | Spec |
|---|---|---|
| `bin/detect-context.sh` (~150 LOC) | routing | [features/routing/tech.md](features/routing/tech.md) §detect-context.sh |
| `bin/set-phase.sh` (~30 LOC) | routing | [features/routing/tech.md](features/routing/tech.md) §set-phase.sh |
| `commands/code-quick.md` (~50 LOC) | commands | [features/commands/](features/commands/tech.md) |
| `hooks/session-start.sh` (~40 LOC) | hooks | [features/hooks/tech.md](features/hooks/tech.md) §session-start.sh |
| Initial `settings.json` registering only SessionStart + bash permissions | hooks | [features/hooks/tech.md](features/hooks/tech.md) §settings.json |

**Done when:**
- `bash bin/detect-context.sh none` returns valid JSON in all four major states (no .spec, .spec without specs, .spec with specs, with active phase).
- `bash bin/set-phase.sh quick` writes `.spec/.phase`; `bash bin/set-phase.sh garbage:state` exits non-zero.
- `/code:quick "fix typo in README"` runs end-to-end on a sandbox project.
- SessionStart prints phase and top lessons to stderr.

---

## M2: Enforcement

**Goal:** Gentle phase gate live. `/code:strategy` produces global specs. Three hard blocks fire correctly.

**Sessions:** 1 | **Risk:** Med — block conditions must be tuned to avoid over- or under-blocking.

| Task | Feature | Spec |
|---|---|---|
| `hooks/pre-tool-use.sh` (~100 LOC) | hooks | [features/hooks/tech.md](features/hooks/tech.md) §pre-tool-use.sh |
| `commands/code-strategy.md` (~80 LOC) | commands | [features/commands/](features/commands/tech.md) |
| Update `settings.json` to register PreToolUse with `Edit\|Write\|NotebookEdit` | hooks | [features/hooks/tech.md](features/hooks/tech.md) §settings.json |
| Test enforcement on sandbox: hard blocks fire, warnings inform | — | — |

**Done when:**
- `/code:strategy` on a fresh project produces `.spec/product.md` and `.spec/tech.md` matching templates.
- `/code:strategy` re-runs without clobbering user-written branch docs.
- Three hard blocks fire correctly; no false positives in normal `/code:quick` use.
- `validate.sh` passes after a strategy run.

---

## M3: Lifecycle

**Goal:** Full `/code:feature` lifecycle. Merge tooling. Remaining two hooks. Installer. v1 feature-complete.

**Sessions:** 1 | **Risk:** Med — `/code:feature` is the longest command; `merge-feature.sh` must be careful.

| Task | Feature | Spec |
|---|---|---|
| `commands/code-feature.md` (~150 LOC) | commands | [features/commands/](features/commands/tech.md) |
| `bin/merge-feature.sh` (~60 LOC) | commands | [features/commands/tech.md](features/commands/tech.md) §merge-feature.sh |
| `hooks/user-prompt-submit.sh` (~50 LOC) | hooks | [features/hooks/tech.md](features/hooks/tech.md) §user-prompt-submit.sh |
| `hooks/stop.sh` (~40 LOC) | hooks | [features/hooks/tech.md](features/hooks/tech.md) §stop.sh |
| `claude/CLAUDE.md` (~60 LOC) | (basic) | [tech.md](tech.md) §Basic Implementation |
| `install.sh` (~80 LOC) | (basic) | [tech.md](tech.md) §Basic Implementation |
| `.gitignore` at repo root | (basic) | — |
| End-to-end test: `/code:feature dark-mode` on sandbox React project | — | — |

**Done when:**
- One full feature cycle (DESIGN → COMPOUND) succeeds end-to-end on a sandbox.
- Global `.spec/tech.md` after COMPOUND shows the merged section (and only the merged section).
- All four hooks active simultaneously without latency complaints.
- `install.sh` works on a fresh machine — produces a working setup with one command.

---

## v1.1: Amend + Config

Deferred. Build only after v1 has been used on at least two real projects.

| Task | Feature |
|---|---|
| `commands/code-amend.md` (~60 LOC) | commands |
| `.shards/config.json` schema + reader | routing |

**Done when:** the use cases that motivated each are demonstrated on a real project, not invented.

---

## Critical Path

```
M1 (Foundation) → M2 (Enforcement) → M3 (Lifecycle) → [dogfood] → v1.1
                                                              ↑
                                                              └─ deferred until needs surface
```

Each milestone is single-session-shaped and independently dogfoodable. M1 alone gives a working `/code:quick`. M1+M2 gives quick + strategy. M1+M2+M3 is v1 complete.

---

## Validation Criteria

### Per milestone
See "Done when:" under each milestone above.

### v1 complete (end of M3)
- All three commands work end-to-end on a sandbox project.
- All four hooks fire at the right times with stderr output that's helpful, not noisy.
- `bin/detect-context.sh` outputs valid JSON for every (workflow, phase) combination.
- `bin/set-phase.sh` validates and writes atomically.
- The three hard blocks fire correctly.
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

| Component | Feature | Milestone | Status |
|---|---|---|---|
| `bin/detect-context.sh` | routing | M1 | NOT STARTED |
| `bin/set-phase.sh` | routing | M1 | NOT STARTED |
| `commands/code-quick.md` | commands | M1 | NOT STARTED |
| `hooks/session-start.sh` | hooks | M1 | NOT STARTED |
| `hooks/pre-tool-use.sh` | hooks | M2 | NOT STARTED |
| `commands/code-strategy.md` | commands | M2 | NOT STARTED |
| `commands/code-feature.md` | commands | M3 | NOT STARTED |
| `bin/merge-feature.sh` | commands | M3 | NOT STARTED |
| `hooks/user-prompt-submit.sh` | hooks | M3 | NOT STARTED |
| `hooks/stop.sh` | hooks | M3 | NOT STARTED |
| `claude/CLAUDE.md` | (basic) | M3 | NOT STARTED |
| `install.sh` | (basic) | M3 | NOT STARTED |
| `commands/code-amend.md` | commands | v1.1 | DEFERRED |
| `.shards/config.json` | routing | v1.1 | DEFERRED |
