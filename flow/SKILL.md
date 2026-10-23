---
name: vibe
description: |
  Vibe workflow router — one skill for the whole flow. Routes to the right phase
  for the current state: setup, strategy, feature, quick, verify, compound.
  Trigger on: any vibe flow state, "what should I do next", flow navigation, set up
  vibe, project direction/strategy, build a feature, quick fix, verify/evidence,
  compound/wrap up, or user says vibe.
user-invocable: true
argument-hint: "[setup|strategy|feature|quick|verify|compound]"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "1.0"
---

# vibe — workflow router

One skill for the whole vibe flow. This lean entry carries the routing table and
the machine-extractable per-turn orders; each phase file holds the full procedure.
Read the phase file for your current (or target) state, then follow it.

## Routing

| Argument | Phase file | States |
|---|---|---|
| `setup` | [setup.md](setup.md) | `setup.detect`, `setup.apply` |
| `strategy` | [strategy.md](strategy.md) | `strategy.brainstorm`, `strategy.spec` |
| `feature` | [feature.md](feature.md) | `feature.design`, `feature.plan`, `feature.impl` |
| `quick` | [quick.md](quick.md) | `quick.triage`, `quick.fix`, `quick.verify` |
| `verify` | [verify.md](verify.md) | `feature.verify`, `quick.verify` |
| `compound` | [compound.md](compound.md) | `feature.compound` |

`scripts/` and `reference/` back the setup phase: `merge-agents.sh`, the `AGENTS.md`
template, and `adapters.json`.

## Style

Output density is governed by one machine-level note (`style` in
[state-machine.json](state-machine.json)), not a per-state level: no filler or
hedging, compress receipts and subagent summaries, but security warnings and
irreversible-action confirmations always stay in full prose, and compression
never reduces reasoning depth — code, paths, and commands stay byte-exact.

## Precedence

The cursor owns sequencing and artifact destinations; delegates own method.
When a delegated skill's text names its own artifact path, commits its own
output, or hands off to another skill, the current state's orders win: write
to the state's surface, leave commits to the flow, transition only via
`set-state.sh`. Scope edits are not a state: edit within the current state's
write surface and stay put. `set-state.sh idle` is always legal — abort ends
any flow.

The two gated edges (`feature.plan>feature.impl`, `feature.verify>feature.compound`
in the machine's `gates`) stop and ask. Via the `/flow` command they apply only
with an explicit `confirm` token in the same message (e.g. `/flow feature.impl
confirm`); without it the command stops and asks. `/flow idle` is never gated.

## Model tiers

Subagent dispatches always name a model tier — mechanical/exploration → sonnet;
review/architecture/synthesis → opus — never an inherited default. Every
`> **Delegate —**` block that dispatches a subagent (Task) pins the tier; copy the
tier into the subagent prompt, since subagents get no per-turn orders.

## Doctrine (SessionStart)

The working-model doctrine, single-sourced here for `scripts/doctrine.sh` (the
only reader — used by the `SessionStart` hook, +`compact` re-inject). The
`AGENTS.md` managed instructions state the same write-invariant + gate policy in
longer prose; a discriminating parity test (`flow/tests/run.sh`) fails if the two
disagree on which states may write what. So the hook makes the `AGENTS.md` block an
optional adapter rather than the only carrier. `doctrine.sh` appends a live cursor
summary after this block.

<!-- vibe:doctrine -->
vibe flow — working model. The spec root (`docs/spec/`, or the legacy dot-spec root) is durable memory; sessions are ephemeral. On session start, Read `lessons.md` and `plan.md` there, then the active feature's `docs/superpowers/{specs,plans}/*-<name>*` (fallback: `docs/spec/features/<name>/{product,tech,plan}.md`). A missing `state.json` cursor means idle — not an error.

You drive the flow: each turn's orders name the current state and its transition command. At a non-gated edge, run `set-state.sh <next>` yourself — nothing advances on its own.

Two human gates — stop and get explicit approval before crossing: plan → impl, and verify → ship. Everything else auto-advances.

Write invariants (`detect-context.sh decide`; default idle when `state.json` is absent): `docs/spec/lessons.md` only in feature.compound, setup.apply, strategy.spec, quick.verify; root `docs/spec/{product,tech,design,plan}.md` only in strategy.spec, feature.compound, setup.apply; `.agents/skills/vibe/state.json` only via `set-state.sh`.

Output: brief technical English (ASD-STE100) — answer first, one idea per sentence, active voice, no filler or hedging; compress receipts and subagent summaries. Never compress security warnings, irreversible-action confirmations, or code/paths/commands — those stay full and byte-exact.
<!-- /vibe:doctrine -->

## Orders (D12)

Machine-extractable per-state orders. The `UserPromptSubmit` inject hook resolves
the current `<flow>.<phase>`, follows its `skill` link (now always `vibe`), and emits
the matching block verbatim via `.agents/skills/vibe/scripts/orders.sh`. `<feature>` is the
only interpolation; keep each block byte-stable.

<!-- vibe:orders:setup.detect -->
skill=vibe · READ-ONLY audit of repo + harness · report what is missing/present (AGENTS.md health, adapter rows from adapters.json, .agents/skills/vibe scripts, vibe skill, bundled spec) · preflight required plugins · do NOT write yet · done → set-state.sh setup.apply
<!-- /vibe:orders -->

<!-- vibe:orders:setup.apply -->
skill=vibe · WRITE/MERGE bootstrap: merge-agents.sh AGENTS.md instructions block, .agents/skills/vibe scaffold, baseline docs/spec/**, optional adapter symlinks (user-driven) · NEVER clobber existing content (diff + ask on divergence) · regen-active-rules.sh after merge · done → set-state.sh idle
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.brainstorm -->
skill=vibe · delegate superpowers:brainstorming · READ the spec root's lessons.md first · scratch only, no source, no spec writes yet · done → set-state.sh strategy.spec
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.spec -->
skill=vibe · delegate spec · WRITE root docs/spec/{product,tech,design,plan}.md · validate after · if a durable lesson surfaced: append tagged docs/spec/lessons.md + regen-active-rules.sh before idle · no source · done → set-state.sh idle | iterate: strategy.brainstorm
<!-- /vibe:orders -->

<!-- vibe:orders:feature.design -->
skill=vibe · delegate superpowers:brainstorming (topic=<feature>) · READ lessons.md + root product/tech · WRITE docs/superpowers/specs/*-<feature>-design.md (fallback: docs/spec/features/<feature>/{product,tech,design?,research?}.md) · no source · done → set-state.sh feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.plan -->
skill=vibe · delegate superpowers:writing-plans · WRITE docs/superpowers/plans/*-<feature>.md (fallback: docs/spec/features/<feature>/plan.md with STABLE unit IDs <feature>/n) · verification per task · no source · HUMAN GATE before impl · gate: plan-approval+mode · on approval → /flow feature.impl confirm
<!-- /vibe:orders -->

<!-- vibe:orders:feature.impl -->
skill=vibe · delegate executing-plans (interactive, default) | subagent-driven-development (handover) + TDD + receiving-code-review on verify-routed re-entry · WRITE src/**, tests/** · do NOT edit docs/spec/** or docs/superpowers/specs/** · cite plan task numbers (fallback: unit IDs <feature>/n) in tests/commits · done → set-state.sh feature.verify
<!-- /vibe:orders -->

<!-- vibe:orders:feature.verify -->
skill=vibe · delegate verification-before-completion + requesting-code-review + code-reviewer (systematic-debugging on fail) · gather EVIDENCE per plan unit ID · no spec writes · HUMAN GATE before ship · gate: ship-approval · on ship → /flow feature.compound confirm (pass) | set-state.sh feature.impl (fix) | feature.plan (drift)
<!-- /vibe:orders -->

<!-- vibe:orders:feature.compound -->
skill=vibe · WRITE tagged docs/spec/lessons.md, promote durable facts from docs/superpowers/{specs,plans}/*-<feature>* + cross-cutting decisions to root specs, git rm them, archive docs/spec/features/<feature> -> docs/spec/archive/<feature> · regen-active-rules.sh · offer archive delete after validation · delegate finishing-a-development-branch LAST (it merges) · done → set-state.sh idle
<!-- /vibe:orders -->

<!-- vibe:orders:quick.triage -->
skill=vibe · READ the spec root's lessons.md first · defect: delegate superpowers:systematic-debugging (diagnose only, no fix) | non-defect: self-scope, no delegate · escalation to feature.design: announce AND confirm · done → set-state.sh quick.fix
<!-- /vibe:orders -->

<!-- vibe:orders:quick.fix -->
skill=vibe · delegate TDD + receiving-code-review on verify-routed re-entry · WRITE src/** (+ optional docs/spec/quick/<slug>.md note) · no root spec writes · done → set-state.sh quick.verify
<!-- /vibe:orders -->

<!-- vibe:orders:quick.verify -->
skill=vibe · delegate verification-before-completion + code-reviewer · gather EVIDENCE the fix works and breaks nothing · no root spec writes · if a durable lesson surfaced: append tagged docs/spec/lessons.md + regen-active-rules.sh before idle · findings → set-state.sh quick.fix | else → set-state.sh idle
<!-- /vibe:orders -->
