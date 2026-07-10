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

2. **Design** (`feature.design`). Follow the spec skill
   [feature.md authoring flow](.agents/skills/spec/feature.md) steps 1–4:
   locate & name → interview WHAT → rigor gate → sketch HOW. Read
   `.spec/lessons.md` first.

   > **Delegate — superpowers:brainstorming**
   > - announce: "delegating to `superpowers:brainstorming` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: spec feature.md steps 1–4 as the dialogue format (RFC-2119 WHAT, GWT scenarios, Scope-table boundaries)
   > - inject: batch clarifying questions into ONE organized list per round (feature-dev Phase 3 pattern); take a SINGLE design approval at the end — no one-question-per-message, no per-section approvals; the flow's only gates are plan and ship
   > - redirect: design doc → `.spec/features/<name>/{product,tech,design}.md` (design.md only at full rigor)
   > - skip: its terminal write under `docs/superpowers/specs/`, its self-commit, its writing-plans handoff

   > **Delegate — code-explorer / code-architect**
   > - announce: "dispatching `code-explorer` (trace) + `code-architect` (sketch) — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: spec feature.md step 4 (which `tech.md` sections to populate, where merge markers go); text-only. Optionally dispatch 2–3 `code-architect` runs in PARALLEL (feature-dev's own pattern — each commits to ONE approach; compare, pick)
   > - redirect: HOW → `.spec/features/<name>/tech.md` (+ `design.md` at full); the subagents write no files themselves
   > - skip: any self-commit or chain handoff. COPY these redirect/skip lines into each subagent (Task) prompt — subagents get no per-turn orders

   Write **only** `.spec/features/<name>/product.md`, `tech.md`, and `design.md`
   when the rigor gate says full. When HOW is sketched, advance to `feature.plan`
   and continue.

3. **Plan** (`feature.plan`). Follow [feature.md](.agents/skills/spec/feature.md)
   step 5 (plan units).

   > **Delegate — superpowers:writing-plans**
   > - announce: "delegating to `superpowers:writing-plans` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: the hybrid plan template (`spec/reference/templates/feature-plan.md`); stable-ID rules (`<name>/n`, never renumber on reorder/split, same-feature deps only); the feature's `product.md` requirements
   > - redirect: plan → `.spec/features/<name>/plan.md` (writing-plans' documented optional storage-location seam)
   > - skip: its own `docs/superpowers/plans/` path; its "offer execution" handoff — the plan gate owns advancing to impl

   `code-architect` may sketch the unit decomposition (text-only, same redirect).
   Write `.spec/features/<name>/plan.md` with **stable unit IDs**; IDs never change
   on reorder or split. **Human gate:** confirm the plan before impl.

   At the plan gate, also **establish the working branch**: `git checkout -b
   feature/<name>` (or confirm an existing non-default branch). Both impl modes and
   the `feature.compound` branch close-out assume it; never implement on the default
   branch without explicit consent (both executors carry their own main-branch
   guard). In **handover** mode ONLY, additionally OFFER an isolated worktree
   (`superpowers:using-git-worktrees`' consent-first pattern — offer, honor a
   standing decline, never impose); the default stays the current branch, and verify
   runs against the same tree either way.

4. **Impl** (`feature.impl`). Mode is picked at the plan gate;
   default **interactive**. Both modes consume the hybrid
   `.spec/features/<name>/plan.md` (units in Seq order; per-unit **Steps** are the
   task checklist) and run on the **feature branch established at the plan gate**
   (worktree only if offered+accepted in handover mode), so verify and the receipt
   run against the same tree. `superpowers:test-driven-development`
   applies within either mode (one reproducing test first, then the minimal code).
   Write `src/**` and `tests/**` only — do **not** edit `.spec/**`. When re-entering
   impl from verify-routed findings, apply `superpowers:receiving-code-review` —
   verify each finding against the codebase before implementing; push back with
   technical reasoning if a finding is wrong.

   **interactive** (default) — drive the plan turn by turn:

   > **Delegate — superpowers:executing-plans**
   > - announce: "delegating to `superpowers:executing-plans` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: the hybrid `.spec/features/<name>/plan.md` as its plan input (units in Seq order; per-unit **Steps** are the task checklist); work on the feature branch from the plan gate, no worktree
   > - redirect: write `src/**` and `tests/**` only; cite plan unit IDs (`<name>/n`) in commits
   > - skip: its `finishing-a-development-branch` exit handoff — the flow advances to `feature.verify` instead

   **handover** — dispatch the plan to subagents:

   > **Delegate — superpowers:subagent-driven-development**
   > - announce: "delegating to `superpowers:subagent-driven-development` — say *self* to keep it inline" — proceed without waiting; self-execute from this file if declined/absent; `suggest-superpowers: false` (.spec/.config.yaml) = standing decline
   > - inject: the hybrid `.spec/features/<name>/plan.md`; work on the feature branch from the plan gate (worktree only if offered+accepted); runtime artifacts stay under `.superpowers/**` (gitignored runtime, not memory)
   > - redirect: write `src/**` and `tests/**` only; cite plan unit IDs (`<name>/n`) in commits. COPY these redirect/skip lines into every subagent (Task) prompt — subagents get no per-turn orders
   > - skip: its `finishing-a-development-branch` exit — stop after the final review; the flow advances to `feature.verify`, which audits regardless

5. **Verify** (`feature.verify`). Hand to `verify.md` /
   `superpowers:verification-before-completion` + `requesting-code-review` +
   `code-reviewer`. Gather evidence per unit ID. **Human gate** before ship.
   Route: pass → `feature.compound`; targeted fix → `feature.impl`; major drift →
   `feature.plan`.

6. **Compound** (`feature.compound`). Hand to
   `compound.md`: record a tagged lesson, promote cross-cutting decisions to
   root specs, archive the feature folder, regenerate the active-rules digest.

## Rules

- Inject exact output paths in every delegation.
- At a non-gated edge, advance immediately: `set-state.sh <next>`, announce in
  one line, continue. Stop and ask only at a `gates` edge (see state-machine.json):
  the two human gates are plan approval before impl and ship approval after verify.
- If a `quick` task escalated here, start at `feature.design`.
- Output density follows the machine's `style` note (see [SKILL.md](SKILL.md)
  § Style): it compresses output, never reasoning. Security/irreversible actions
  stay in normal prose.
