---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
children: []
updated: 2026-06-04
---

# vibe — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md)
**Features:** [features/spec-framework/](features/spec-framework/product.md),
[features/vibe-flow/](features/vibe-flow/product.md),
[features/platform-adapters/](features/platform-adapters/product.md)

---

## Validation Summary

Stage 1 is built: the bundled `spec` skill, the root `.spec/` docs, the
`.agents/flow` state machine + scripts, the seven `vibe-*` skill shims, and the
Codex/Claude adapters all exist. Superseded prior art (the engineering-agent
import and the pre-vibe-flow command/hook/routing/strict-flow experiments) has
been removed from `.spec/archive/`; the repo now reflects only the current
design.

The core design decision this rests on is:

> `spec` owns durable planning in `.spec/`; `vibe-*` skills own agent workflow
> orchestration; `.agents/flow` owns platform-neutral runtime state.

---

## Critical Architecture Decisions

### Decided

- **Spec framework remains independent.** It can be used without the vibe flow.
- **Root spec model is product, tech, design, plan, lessons.**
- **Feature spec model is product and tech required; design, plan, research optional.**
- **Vibe workflow shims are agent skills.** They live under `.agents/skills/vibe-*`.
- **Canonical flow state is platform-neutral.** Use `.agents/flow`, not `.spec/.phase` or `.claude/state.json`.
- **Adapters are thin.** `AGENTS.md`, `CLAUDE.md`, and `.claude/*` read the same `.agents/flow` core.
- **States are compound `<flow>.<phase>` keys.** Transitions and `next` arrays key on the compound state, not bare `phase`. The cursor drops the `notes` field.
- **D8: Lessons are retrievable.** Tagged entries in `lessons.md`, read on entry to `*.design` and `*.triage`. KISS — one file, keyword scan, no schema.
- **D9: Stable plan unit IDs.** `feature.plan` assigns `U1`, `U2`, …; `impl`/`verify` cite them so state survives re-planning.
- **D10: One inject owner.** A single `UserPromptSubmit` inject per state names skill/writes/path/caveman/next and sets caveman; safety carve-outs override density; nothing turn-varying enters the inject (prompt-cache discipline).
- **D12: Skill shim is the inject source (supersedes D10's frozen-string mechanism).** The per-turn orders live in each `vibe-*` skill, not in a hand-written `state-machine.json` `inject` string. The state's `skill` field links it to its shim; the `UserPromptSubmit` hook pulls that skill's per-`<flow>.<phase>` orders block and injects it. D10's invariants survive — one inject owner, nothing turn-varying, cache-stable. Skill-less states (`idle`, `amend`) keep a minimal inline fallback string. Implemented when the skill shims are updated; see [features/platform-adapters/plan.md](features/platform-adapters/plan.md) (U8 + U1).
- **D11: Cherry-pick feature-dev subagents.** `code-explorer`/`code-architect` into `feature.design`, `code-reviewer` into `*.verify`; not the `/feature-dev` macro.
- **Caveman provenance.** Levels follow `JuliusBrussee/caveman`; the phase→level mapping is ours. `ultra` is not used for triage.

### To Resolve

- [x] **OPEN-1: Mutable state tracking.** Resolved: gitignore the mutable cursor
  `.agents/flow/state.json`; version the static `state-machine.json` and the
  `state.example.json` template. (See `.gitignore`.)
- [ ] **OPEN-2: Skill count.** Validate whether `vibe-verify` and `vibe-compound`
  should stay separate skills, and whether `vibe-setup` should also own install
  repair after initial bootstrap.
- [ ] **OPEN-6: R7 graceful degradation.** Port the archived detect-and-drop+warn
  skill-availability check into `.agents/flow/scripts/`, or downgrade R7 to a hard
  "require superpowers+spec, fail loudly" prerequisite. Include a 1-line caveman
  fallback when the caveman plugin is absent.
- [ ] **OPEN-7: Enforcement strictness.** Does "strict workflow" mean a real
  PreToolUse block, or model-read prose only? Decide before M4 hooks.
- [ ] **D7 (deferred): `feature.deepen`.** Optional confidence-gated deepen pass
  between `plan` and `impl`. Revisit after the base feature arc is dogfooded.
- [ ] **OPEN-3: Adapter install mode.** Symlink, copy, or merge-with-diff per file?
- [ ] **OPEN-4: Hook strictness.** Which write surfaces should adapters block in
  M1 versus warn about?
- [ ] **OPEN-5: DESIGN.md validation.** Should `spec validate` shell out to
  `npx @google/design.md lint` when visual tokens are present, or keep local
  validation dependency-free?

---

## Implementation Roadmap

| Milestone | Goal | Sessions | Risk |
|---|---|---:|---|
| M0: Spec cleanup | Update specs and spec skill to the new `spec + vibe flow + adapters` model | 1 | Low |
| M1: Flow core | Add `.agents/flow` state machine and deterministic scripts | 1 | Medium |
| M2: Vibe skills | Add `vibe-setup`, `vibe-strategy`, `vibe-feature`, and `vibe-quick` skills | 1-2 | Medium |
| M3: Verification and compound | Add `vibe-verify`, `vibe-compound`, `vibe-amend` and evidence/lessons flow | 1 | Medium |
| M4: Platform adapters | Rewrite `AGENTS.md`, `CLAUDE.md`, Claude commands/hooks, installer | 1-2 | Medium |
| M5: Dogfood | Run strategy, quick, and feature flows on a sandbox project | 1 | High |

---

## M0: Spec Cleanup

**Goal:** The repository specs consistently describe the current architecture.

**Tasks:**

- [x] Update `spec` skill to include root `design.md`.
- [x] Make setup/validation scripts work from a vendored skill path.
- [x] Rewrite root product and tech specs around `spec`, `vibe flow`, and adapters.
- [x] Add feature specs for `spec-framework`, `vibe-flow`, and `platform-adapters`.
- [x] Retire or archive stale `commands`, `routing`, `hooks`, and `strict-flow` specs.
- [x] Validate `.spec/`.

**Done when:** validation passes and no root spec names `.spec/.phase` or
`.claude/state.json` as canonical.

---

## M1: Flow Core

**Goal:** Platform-neutral flow state exists and can be read/written safely.

**Tasks:**

- [x] Add `.agents/flow/state-machine.json` (all states from §4, frozen injects).
- [x] Add `.agents/flow/state.example.json`.
- [x] Add `.agents/flow/scripts/detect-context.sh` (snapshot + allow/warn/block
  decision fn — the one place the invariant policy lives).
- [x] Add `.agents/flow/scripts/set-state.sh` (validated atomic writer).
- [x] Add `.agents/flow/scripts/validate-state.sh`.
- [x] Add `.agents/flow/scripts/regen-active-rules.sh` (capped top-5 digest).
- [x] Document which mutable flow files target projects should gitignore
  (`.agents/flow/state.json`).

**Done when:** state validation passes, legal transitions work, invalid transitions
fail with a clear message, and no `.claude/*` file is required to use the core.

---

## M2: Vibe Skills

**Goal:** The primary workflow surface exists as agent skills.

**Tasks:**

- [x] Add `.agents/skills/vibe-strategy/SKILL.md`.
- [x] Add `.agents/skills/vibe-feature/SKILL.md`.
- [x] Add `.agents/skills/vibe-quick/SKILL.md`.
- [x] Add `.agents/skills/vibe-setup/SKILL.md` (detect→apply, constitution block,
  plugin preflight).
- [x] Add caveman level metadata to the state machine contract (per-state
  `caveman` field + `caveman_levels`/`safety_carveouts` in the machine).
- [x] Keep each skill concise and delegate to `spec`, `superpowers:*`, and
  subagents with explicit path injection.
- [x] Add shared references only if the skill bodies become too large. (Not
  needed — bodies stayed concise.)

**Done when:** each skill can be triggered by description, reads `.agents/flow`,
and names the exact `.spec/` paths delegated skills may write.

---

## M3: Verification and Compound

**Goal:** Completion is evidence-backed and lessons flow back into specs.

**Tasks:**

- [x] Add `.agents/skills/vibe-verify/SKILL.md`.
- [x] Add `.agents/skills/vibe-compound/SKILL.md`.
- [x] Add `.agents/skills/vibe-amend/SKILL.md`.
- [x] Define how failed verification routes back to feature planning or
  implementation (`feature.verify.next` = compound | impl | plan).
- [x] Define how feature lessons promote into root `.spec/lessons.md`
  (vibe-compound + regen-active-rules.sh digest into the adapter blocks).

**Done when:** a feature can move from design to implementation to verification
to lessons/archive without using ad hoc prompts.

---

## Spec Skill Follow-Up

**Goal:** Tighten the spec framework after the flow core exists.

**Tasks:**

- [ ] Add `reference/design.md` with guidance derived from the `DESIGN.md`
  token-plus-prose model.
- [ ] Teach `list-specs.sh` to surface root and feature `design.md` docs.
- [ ] Optionally validate `design.md` token structure when tokens are present.
- [ ] Decide whether to support `npx @google/design.md lint` as an optional
  external validator.

---

## M4: Platform Adapters & Claude Code Plugin

**Goal:** Codex and Claude Code expose the same flow without owning it — and
Claude Code installs it as a plugin whose hooks make the flow automatic.

**Stage 2 unit-level plan:** the buildable breakdown (units `U1`–`U7`, stable
IDs) lives in [features/platform-adapters/plan.md](features/platform-adapters/plan.md).
The milestone checklist below stays the high-level roadmap view.

**Stage 1 (done — guidance only):**

- [x] Rewrite `AGENTS.md` as the Codex-facing adapter policy.
- [x] Rewrite `CLAUDE.md` as the Claude-facing adapter policy (constitution +
  flow-state/transition guidance + generated active-rules block).
- [x] `.claude/commands/flow.md` reads `.agents/flow` and refuses illegal
  transitions.

**Stage 2 (the Claude Code plugin + hooks — earn the teeth):**

- [ ] Add `.claude-plugin/plugin.json` so vibe installs as a Claude Code
  plugin bundling the `/flow` command, the `vibe-*` skills, and the hooks.
- [ ] Add `.claude/hooks/hooks.json` wiring events to scripts via
  `${CLAUDE_PLUGIN_ROOT}`.
- [ ] **Skill-as-inject-source (D12)** prerequisite: give each `vibe-*` skill a
  per-state orders block and relink the state machine (`inject` → `null` for
  skill-owning states; inline fallback only for `idle`/`amend`). See
  features/platform-adapters/plan.md U8.
- [ ] **Inject hook** (`UserPromptSubmit`): resolve the current state's linked
  skill and inject that skill's per-state orders every turn (D12). No exit codes.
  Static-content discipline.
- [ ] **Guard hook** (`PreToolUse` `Edit|Write|NotebookEdit`): exit 2 on the
  three hard blocks, warn elsewhere, via `detect-context.sh decide`.
- [ ] **Gate hook** (`Stop`): warn-first exit-predicate checks (stuck phase,
  impl-without-tests, verify-without-review, forgotten `set-state.sh`).
- [ ] Keep every hook a thin shell over `.agents/flow/scripts/`; no invariant
  logic duplicated. Graceful degrade: missing keystone → exit 0.
- [ ] Build installer behavior for copying/symlinking core + adapter files and
  registering the Claude Code plugin.

**Done when:** both adapters point to `.agents/flow` and `.agents/skills/vibe-*`,
the Claude Code plugin installs command + skills + hooks in one step, the inject
fires every turn, the three invariants are guarded, and neither adapter defines a
separate state model or spec layout. Blocking predicates are promoted only after
M5 dogfooding earns them.

---

## M5: Dogfood

**Goal:** Prove the workflow on a real project shape.

**Tasks:**

- [ ] Run `vibe-strategy` on a sandbox project.
- [ ] Run `vibe-quick` for a small maintenance change.
- [ ] Run `vibe-feature` through verification and compound.
- [ ] Record at least one lesson from actual friction.

**Done when:** the workflow completes without manual path correction and the
adapter wording is understandable in both Codex and Claude Code.

---

## Critical Path

```text
M0 -> M1 -> M2 -> M3 -> M4 -> M5
```

M1 and M2 are tightly coupled: the skills need a stable state contract. M4 should
wait until the core flow stops moving.

---

## Progress

| Milestone | Status | Sessions Used | Estimate |
|---|---|---:|---:|
| M0: Spec cleanup | DONE | 1 | 1 |
| M1: Flow core | DONE | 1 | 1 |
| M2: Vibe skills | DONE | 1 | 1-2 |
| M3: Verification and compound | DONE | 1 | 1 |
| M4: Platform adapters | PARTIAL | 1 | 1-2 |
| M5: Dogfood | NOT STARTED | 0 | 1 |

**Stage 1 complete** (per the design conclusion §10): the flow runs end-to-end on
guidance alone — state machine as data, deterministic scripts, seven `vibe-*`
skill shims, constitution wiring in both adapters, and the generated active-rules
digest. Hooks (UserPromptSubmit inject, PreToolUse guard, Stop gate — §11 Stage 2)
are intentionally deferred until M5 dogfooding validates the flow shape and counts
overrides. M4 remains partial: adapter prose + `/flow` command done; hooks and an
installer are the Stage-2 remainder.
