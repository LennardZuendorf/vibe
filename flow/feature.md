# feature — build loop

Carries a named feature from spec to verified code. Five states with **two human
gates**: after `plan` (before impl) and after `verify` (before ship/compound).
Everything between is autonomous.

`feature.design → feature.plan →[gate] feature.impl → feature.verify →[gate]
(feature.compound | feature.impl | feature.plan)`

## Procedure

1. **Locate + name.** Read `.agents/skills/vibe/state.json`. Establish the feature name
   and set it: `bash .agents/skills/vibe/scripts/set-state.sh feature.design <name>`.
   The name is carried in the cursor across all feature states.

2. **Design** (`feature.design`, caveman lite). Follow the spec skill
   [feature.md authoring flow](.agents/skills/spec/feature.md) steps 1–4:
   locate & name → interview WHAT → rigor gate → sketch HOW. Read
   `.spec/lessons.md` first.

   > **Delegate — superpowers:brainstorming**
   > - announce: "delegating to `superpowers:brainstorming` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: spec feature.md steps 1–4 as the dialogue format (RFC-2119 WHAT, GWT scenarios, Scope-table boundaries); caveman lite
   > - redirect: design doc → `.spec/features/<name>/{product,tech,design}.md` (design.md only at full rigor)
   > - skip: its terminal write under `docs/superpowers/specs/`, its self-commit, its writing-plans handoff

   > **Delegate — code-explorer / code-architect**
   > - announce: "dispatching `code-explorer` (trace) + `code-architect` (sketch) — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: spec feature.md step 4 (which `tech.md` sections to populate, where merge markers go); text-only. Optionally dispatch 2–3 `code-architect` runs in PARALLEL (feature-dev's own pattern — each commits to ONE approach; compare, pick)
   > - redirect: HOW → `.spec/features/<name>/tech.md` (+ `design.md` at full); the subagents write no files themselves
   > - skip: any self-commit or chain handoff. COPY these redirect/skip lines into each subagent (Task) prompt — subagents get no per-turn orders

   Write **only** `.spec/features/<name>/product.md`, `tech.md`, and `design.md`
   when the rigor gate says full. Suggest `feature.plan` when HOW is sketched.

3. **Plan** (`feature.plan`, caveman lite). Follow [feature.md](.agents/skills/spec/feature.md)
   step 5 (plan units).

   > **Delegate — superpowers:writing-plans**
   > - announce: "delegating to `superpowers:writing-plans` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: the hybrid plan template (`spec/reference/templates/feature-plan.md`); stable-ID rules (`<name>/n`, never renumber on reorder/split, same-feature deps only); the feature's `product.md` requirements
   > - redirect: plan → `.spec/features/<name>/plan.md` (writing-plans' documented optional storage-location seam)
   > - skip: its own `docs/superpowers/plans/` path; its "offer execution" handoff — the plan gate owns advancing to impl

   `code-architect` may sketch the unit decomposition (text-only, same redirect).
   Write `.spec/features/<name>/plan.md` with **stable unit IDs**; IDs never change
   on reorder or split. **Human gate:** confirm the plan before impl.

4. **Impl** (`feature.impl`, caveman full). Delegate to
   `superpowers:executing-plans` + `superpowers:test-driven-development`. Write
   `src/**` and `tests/**` only — do **not** edit `.spec/**`. Cite plan unit IDs
   in tests/commits so state survives re-planning.

5. **Verify** (`feature.verify`, caveman full). Hand to `verify.md` /
   `superpowers:verification-before-completion` + `requesting-code-review` +
   `code-reviewer`. Gather evidence per unit ID. **Human gate** before ship.
   Route: pass → `feature.compound`; targeted fix → `feature.impl`; major drift →
   `feature.plan`.

6. **Compound** (`feature.compound`, caveman lite; receipts ultra). Hand to
   `compound.md`: record a tagged lesson, promote cross-cutting decisions to
   root specs, archive the feature folder, regenerate the active-rules digest.

## Rules

- Inject exact output paths in every delegation.
- Transitions are agent-*suggested* until proven: name the next state, confirm.
- If a `quick` task escalated here, start at `feature.design`.
- Caveman compresses output, never reasoning. Security/irreversible actions stay
  in normal prose at every level.
