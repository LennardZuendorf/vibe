---
name: flow
description: Use when building, fixing, refactoring, planning implementation, resuming work, verifying a candidate, or running /flow. Owns lifecycle sequencing and bounded task delegation. Direct spec knowledge operations use spec; toolkit orientation uses vibe.
user-invocable: true
argument-hint: "[setup|strategy|feature|quick|verify|compound]"
allowed-tools: Read, Edit, Write, Bash
compatibility: Requires bash + jq. macOS and Linux.
metadata:
  author: lennarddib
  version: "2.0"
---

<!-- vibe-spec: literal-file -->

# Vibe Flow

Flow owns lifecycle sequencing, task dispatch and review. Spec owns artifact
operations. External methods support one bounded step; they do not own this run.

## Entry and discovery

A delegated worker follows its TaskBrief and returns a TaskResult; it never
restarts this router. Direct `$flow` and `vibe:flow` need no toolkit bootstrap.
Discover the spec using `vibe-spec root`; read its plan and relevant lessons.
Without the binary, inspect an existing `docs/spec/` or `.spec/` read-only and <!-- literal -->
report validation unavailable. With no root, offer explicitly requested Spec
scaffolding before feature work. Missing Flow/Spec resources are a repair
condition; name the missing component and do not install it without a request.
For competing copies, report selected source and shadowing; preserve foreign files.

Read `.vibe/run/flow/state.json` first; otherwise the legacy cursor under
`.agents/skills/flow/state.json`, `.agents/skills/vibe/state.json` or
`.agents/flow/state.json`. Only absence means idle. A present corrupt cursor
requires [recovery](reference/recovery.md). Read the machine and current D12
orders before resuming. Use `vibe-flow state transition` for v2. `state set` and the packaged shell writer
are legacy v1 only and refuse adoption. Never hand-edit a cursor. Existing runtime capabilities are described in
[compatibility](reference/compatibility.md); instructions do not authenticate approval.

## Contract

The controller owns state, progress and review dispatch. Legacy wrapper `confirm`
tokens are conventions, not verified grants. Design and plan freeze
at implementation approval. Record progress outside those frozen artifacts.
Workers receive exact scope, dependencies and checks through the
[task contract](reference/task-contract.md). Worker claims require checked
results and [independent review](reference/review-contract.md).

Human approval binds the reviewed plan and scope before implementation, then the
final candidate and named ship action. Scope/plan edits invalidate approval;
candidate edits invalidate final readiness. See [authorization](reference/authorization.md).
No automatic commit, push, merge, release, cleanup or branch finishing. Do not
invoke whole-lifecycle executors that assume these actions.

## Procedures

Load only the current procedure and needed reference section.

| Intent | Procedure |
| --- | --- |
| Named feature or multi-step change | [feature](procedures/feature.md) |
| At most two files, twenty changed lines, no contract change | [quick](procedures/quick.md) |
| Harness inventory or requested repair | [setup](procedures/setup.md) |
| Project direction or durable spec decisions | [strategy](procedures/strategy.md) |

Choose capability before model. Use controlling user/repository model preferences;
never hardcode a provider tier that overrides them. Native agent registration is
optional: a generic agent can consume the packaged role brief. Missing required
independent review remains `blocked`, never silent self-review.

## Orders (D12)

The machine links to this skill. The blocks below are the single authored home
for current runtime orders; procedures expand the method, not transition data.
The upgraded runtime defines compound-before-verify and ship states. Its strong
human gate checks remain unavailable/deny until native authority is proven.

<!-- vibe:doctrine -->
vibe flow — working model. Read the discovered spec's plan and lessons, then active design/plan. Cursor: `.vibe/run/flow/state.json`; absence means idle only before adoption.

Design → plan → approved impl → additive compound → verify → approved ship. Controller owns tasks and independent review. Workers cannot widen scope or approve themselves.

V2 uses `vibe-flow state transition`; verified human authority remains unavailable/deny. `vibe-flow state set` is legacy v1 only. Check paths with `vibe-flow policy decide`. No automatic commit, push, release, cleanup or finishing. Output: brief technical English; answer first.
<!-- /vibe:doctrine -->

<!-- vibe:orders:setup.detect -->
skill=flow · read procedures/setup.md · audit read-only; report components/sources/shadowing · requested mutation → feature.design; else idle
<!-- /vibe:orders -->

<!-- vibe:orders:setup.apply -->
skill=flow · legacy readable state; no mutation authority · requested repair → feature.design; else idle
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.brainstorm -->
skill=flow · read procedures/strategy.md · clarify direction with bounded brainstorming; lessons first · done → strategy.spec
<!-- /vibe:orders -->

<!-- vibe:orders:strategy.spec -->
skill=flow · bounded Spec root-artifact operation; validate · implementation request → feature.design; else idle
<!-- /vibe:orders -->

<!-- vibe:orders:feature.design -->
skill=flow · read procedures/feature.md · bounded brainstorming + explorer/architect; Spec artifact ownership · design only · done → feature.plan
<!-- /vibe:orders -->

<!-- vibe:orders:feature.plan -->
skill=flow · bounded planning method; design-linked task scopes/dependencies/checks · reviewed plan and scope require verified human approval before impl; authority unavailable means deny
<!-- /vibe:orders -->

<!-- vibe:orders:feature.impl -->
skill=flow · bounded implementer + TDD; controller checks progress and independent review · freeze design/plan; exact approved task paths only · done → feature.compound
<!-- /vibe:orders -->

<!-- vibe:orders:feature.verify -->
skill=flow · real candidate-bound checks + independent final review after docs · no source writes · evidence → .vibe/run/flow/evidence/feature-<feature>.md · verified human approval of candidate + named action required; unavailable means deny ship
<!-- /vibe:orders -->

<!-- vibe:orders:feature.compound -->
skill=flow · additive docs/lessons/rules preparation; retain feature artifacts · no destructive promotion or finishing · done → feature.verify
<!-- /vibe:orders -->

<!-- vibe:orders:quick.triage -->
skill=flow · read procedures/quick.md · diagnose before fix · ≤2 files/20 changed lines/no contract change; growth → feature.design with approval · done → quick.fix
<!-- /vibe:orders -->

<!-- vibe:orders:quick.fix -->
skill=flow · bounded TDD; reproducer red before fix, green after · exact bounded paths; no root spec writes · done → quick.verify
<!-- /vibe:orders -->

<!-- vibe:orders:quick.verify -->
skill=flow · real checks + independent review as required; evidence → .vibe/run/flow/evidence/quick.md · findings → quick.fix; else idle
<!-- /vibe:orders -->

<!-- vibe:orders:feature.ship -->
skill=flow · only the approved named action on the verified candidate · no implicit commit/release/cleanup · successful action → idle
<!-- /vibe:orders -->
