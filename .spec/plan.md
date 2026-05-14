---
type: entrypoint
scope: implementation
covers: milestones, build sequence, validation criteria, open decisions
children: []
updated: 2026-05-14
---

# shards-code — Implementation Plan

**Parent specs:** [product.md](product.md), [tech.md](tech.md), [design.md](design.md)
**Features:** [features/spec-framework/](features/spec-framework/product.md),
[features/code-flow/](features/code-flow/product.md),
[features/platform-adapters/](features/platform-adapters/product.md)

---

## Validation Summary

The current repo is mostly specs and agent-skill scaffolding. The useful existing
assets are:

- Bundled `spec` skill under `.agents/skills/spec/`
- Existing root `.spec/` docs
- Existing platform adapter drafts in `CLAUDE.md` and `.claude/`
- Prior strict-flow experiment, now superseded by the `code` flow direction

The core design decision for this cleanup is:

> `spec` owns durable planning in `.spec/`; `code-*` skills own agent workflow
> orchestration; `.agents/flow` owns platform-neutral runtime state.

---

## Critical Architecture Decisions

### Decided

- **Spec framework remains independent.** It can be used without the code flow.
- **Root spec model is product, tech, design, plan, lessons.**
- **Feature spec model is product and tech required; design, plan, research optional.**
- **Code workflow shims are agent skills.** They live under `.agents/skills/code-*`.
- **Canonical flow state is platform-neutral.** Use `.agents/flow`, not `.spec/.phase` or `.claude/state.json`.
- **Adapters are thin.** `AGENTS.md`, `CLAUDE.md`, and `.claude/*` read the same `.agents/flow` core.

### To Resolve

- [ ] **OPEN-1: Mutable state tracking.** Should target projects gitignore only
  `.agents/flow/state.json`, or all runtime-generated `.agents/flow/*.json` except
  the state machine?
- [ ] **OPEN-2: Skill count.** Validate whether `code-verify` and `code-compound`
  should stay separate skills, and whether `code-setup` should also own install
  repair after initial bootstrap.
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
| M0: Spec cleanup | Update specs and spec skill to the new `spec + code flow + adapters` model | 1 | Low |
| M1: Flow core | Add `.agents/flow` state machine and deterministic scripts | 1 | Medium |
| M2: Code skills | Add `code-setup`, `code-strategy`, `code-feature`, and `code-quick` skills | 1-2 | Medium |
| M3: Verification and compound | Add `code-verify`, `code-compound`, `code-amend` and evidence/lessons flow | 1 | Medium |
| M4: Platform adapters | Rewrite `AGENTS.md`, `CLAUDE.md`, Claude commands/hooks, installer | 1-2 | Medium |
| M5: Dogfood | Run strategy, quick, and feature flows on a sandbox project | 1 | High |

---

## M0: Spec Cleanup

**Goal:** The repository specs consistently describe the current architecture.

**Tasks:**

- [x] Update `spec` skill to include root `design.md`.
- [x] Make setup/validation scripts work from a vendored skill path.
- [x] Rewrite root product and tech specs around `spec`, `code flow`, and adapters.
- [x] Add feature specs for `spec-framework`, `code-flow`, and `platform-adapters`.
- [x] Retire or archive stale `commands`, `routing`, `hooks`, and `strict-flow` specs.
- [x] Validate `.spec/`.

**Done when:** validation passes and no root spec names `.spec/.phase` or
`.claude/state.json` as canonical.

---

## M1: Flow Core

**Goal:** Platform-neutral flow state exists and can be read/written safely.

**Tasks:**

- [ ] Add `.agents/flow/state-machine.json`.
- [ ] Add `.agents/flow/state.example.json`.
- [ ] Add `.agents/flow/scripts/detect-context.sh`.
- [ ] Add `.agents/flow/scripts/set-state.sh`.
- [ ] Add `.agents/flow/scripts/validate-state.sh`.
- [ ] Document which mutable flow files target projects should gitignore.

**Done when:** state validation passes, legal transitions work, invalid transitions
fail with a clear message, and no `.claude/*` file is required to use the core.

---

## M2: Code Skills

**Goal:** The primary workflow surface exists as agent skills.

**Tasks:**

- [ ] Add `.agents/skills/code-strategy/SKILL.md`.
- [ ] Add `.agents/skills/code-feature/SKILL.md`.
- [ ] Add `.agents/skills/code-quick/SKILL.md`.
- [ ] Add `.agents/skills/code-setup/SKILL.md`.
- [ ] Add caveman level metadata to the state machine contract.
- [ ] Keep each skill concise and delegate to `spec`, `superpowers:*`, and
  subagents with explicit path injection.
- [ ] Add shared references only if the skill bodies become too large.

**Done when:** each skill can be triggered by description, reads `.agents/flow`,
and names the exact `.spec/` paths delegated skills may write.

---

## M3: Verification and Compound

**Goal:** Completion is evidence-backed and lessons flow back into specs.

**Tasks:**

- [ ] Add `.agents/skills/code-verify/SKILL.md`.
- [ ] Add `.agents/skills/code-compound/SKILL.md`.
- [ ] Add `.agents/skills/code-amend/SKILL.md`.
- [ ] Define how failed verification routes back to feature planning or
  implementation.
- [ ] Define how feature lessons promote into root `.spec/lessons.md`.

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

## M4: Platform Adapters

**Goal:** Codex and Claude Code expose the same flow without owning it.

**Tasks:**

- [ ] Rewrite `AGENTS.md` as the Codex-facing adapter policy.
- [ ] Rewrite `CLAUDE.md` as the Claude-facing adapter policy.
- [ ] Update `.claude/commands/flow.md` to read/write `.agents/flow`.
- [ ] Add optional Claude hooks that read `.agents/flow`.
- [ ] Build installer behavior for copying or symlinking core and adapter files.

**Done when:** both adapters point to `.agents/flow` and `.agents/skills/code-*`,
and neither defines a separate state model or spec layout.

---

## M5: Dogfood

**Goal:** Prove the workflow on a real project shape.

**Tasks:**

- [ ] Run `code-strategy` on a sandbox project.
- [ ] Run `code-quick` for a small maintenance change.
- [ ] Run `code-feature` through verification and compound.
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
| M1: Flow core | NOT STARTED | 0 | 1 |
| M2: Code skills | NOT STARTED | 0 | 1-2 |
| M3: Verification and compound | NOT STARTED | 0 | 1 |
| M4: Platform adapters | NOT STARTED | 0 | 1-2 |
| M5: Dogfood | NOT STARTED | 0 | 1 |
