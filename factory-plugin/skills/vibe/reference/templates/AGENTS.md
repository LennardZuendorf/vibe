# AGENTS.md — vibe Engineering Guide

<!-- vibe:instructions:start -->
<!-- Managed by vibe (merge-agents.sh); replaced on setup.apply / install.
     Content outside these markers, including vibe:active-rules, is user-owned. -->
This project runs the **vibe flow**, a spec-first workflow harness. The spec root (`docs/spec/`, or legacy `.spec/`) is durable memory; sessions are ephemeral.
Read the specs before writing code. A flow-only install has no spec tree.
## Session start
1. Read the spec root's `lessons.md` and `plan.md` when present.
2. Load `docs/superpowers/specs/*-<name>-design.md` and `docs/superpowers/plans/*-<name>.md`; fallback: `docs/spec/features/<name>/{product,tech,plan}.md`.
3. Read `{flow, phase, feature}` from `.vibe/run/flow/state.json` when present, otherwise `.agents/skills/vibe/state.json`. Only absence means `idle`; report a present invalid cursor. Resume the linked `vibe` skill.
## Working model
`design → plan →[gate] impl → verify →[gate] compound`
Most transitions auto-advance. Two edges are **human gates**: get explicit
approval before crossing: plan → impl, and verify → ship. Other edges need no pause.
`/flow <flow.phase> [feature] [confirm]` carries the feature; literal `confirm`
crosses a gated edge. Never hand-edit `state.json`. The underlying command is
`bash .agents/skills/vibe/scripts/set-state.sh <flow.phase> [feature]`.
Gates are a convention today: `/flow` asks for a `confirm` token, and no hook or script checks it.
`set-state.sh` writes the cursor; it does not gate. No hook enforces the two approvals; honor them yourself.
## Precedence and policy
The cursor owns sequencing and artifact destinations; delegated skills own method. Follow current-state orders from the linked `vibe` skill, resolved by `orders.sh`,
over delegated prose or artifact paths. Scope edits stay in the current state's
write surface. Transition only via `set-state.sh`; `set-state.sh idle` always aborts.
Restricted paths and states are data in `content/policy.json`, enforced by
`detect-context.sh decide` and rendered rule-by-rule below. Absent cursor means idle.
Standing blocks compose the per-turn prompt, session start and `vibe:rules` channels. Configure them in root `vibe.json`; edit blocks/config, never generated regions.
## Commands
```bash
vibe-spec validate # before claiming done; absent on --only flow
bash .agents/skills/vibe/scripts/detect-context.sh decide <path> # before writes
bash .agents/skills/vibe/scripts/doctor.sh
node .agents/skills/vibe/engine/cli.mjs render --list
node .agents/skills/vibe/engine/cli.mjs render --check
node .agents/skills/vibe/engine/cli.mjs render agents-md --write
```
## Enforcement is partial — do not trust it blindly
The guard intercepts only `Edit`, `Write`, `MultiEdit`, and `NotebookEdit`.
Shell redirection (`>`, `tee`, `sed -i`) bypasses it; a warn-only Bash sniffer backs the convention. No hook enforces either human approval. The Stop receipt
tooth requires fresh evidence in verify only when platform hooks are wired
(e.g. `.claude/settings.json`). No wiring, no tooth.
Scripts graceful-degrade; `jq` is optional (pure bash keeps cursor bytes identical).
Other agents may edit concurrently: touch only your own files; prefer warn over hard-fail.
**Output: brief technical English** — answer first, one idea per sentence, active voice, no filler. Compress receipts/summaries; keep security warnings, irreversible-action
confirmations, code, paths and commands full and byte-exact.
<!-- vibe:instructions:end -->

<!-- vibe:active-rules:start -->
<!-- Generated from lessons.md by regen-active-rules.sh. Do not edit by hand. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->

<!-- vibe:rules -->
_Managed by vibe — rendered from the `agents-md` channel by `node .agents/skills/vibe/engine/cli.mjs render agents-md --write`. Edit the blocks or `vibe.json`, not this region._

## Write invariants

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

- `.vibe/run/flow/state.json`, `.agents/skills/vibe/state.json`, `.agents/flow/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit
- `.spec/lessons.md` — `feature.compound`, `setup.apply`, `strategy.spec`, `quick.verify`: allow; otherwise: block — .spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: the current state)
- `docs/spec/lessons.md` — `feature.compound`, `setup.apply`, `strategy.spec`, `quick.verify`: allow; otherwise: block — docs/spec/lessons.md is writable only during feature.compound, setup.apply, strategy.spec, or quick.verify (current: the current state)
- `.spec/product.md`, `.spec/tech.md`, `.spec/design.md`, `.spec/plan.md` — `strategy.spec`, `feature.compound`, `setup.apply`: allow; otherwise: block — root .spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: the current state)
- `docs/spec/product.md`, `docs/spec/tech.md`, `docs/spec/design.md`, `docs/spec/plan.md` — `strategy.spec`, `feature.compound`, `setup.apply`: allow; otherwise: block — root docs/spec specs are writable only during strategy.spec, feature.compound, or setup.apply (current: the current state)
- `.planning/*` — `idle`: allow; otherwise: block — .planning/ is GSD's state store — vibe reads it as context and never writes it; route edits through GSD's own commands
- `.spec/features/*` — `feature.impl`, `quick.fix`: warn — .spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: the current state); otherwise: allow
- `docs/spec/features/*` — `feature.impl`, `quick.fix`: warn — docs/spec/features edits are frozen during impl/fix — route back to feature.design/plan to change scope (current: the current state); otherwise: allow
- `docs/superpowers/specs/*` — `feature.impl`, `quick.fix`: warn — superpowers design docs are frozen during impl/fix — route back to feature.design to change scope (current: the current state); otherwise: allow
- `CLAUDE.md`, `AGENTS.md` — otherwise: warn — CLAUDE.md/AGENTS.md active-rules block is generated by regen-active-rules.sh; edits inside the markers are overwritten next compound
- `src/*`, `tests/*` — `feature.verify`: warn — verify writes no src — route findings back to impl (set-state.sh feature.impl); `quick.verify`: warn — verify writes no src — route findings back to fix (set-state.sh quick.fix); `feature.impl`, `quick.fix`, `setup.apply`: allow; otherwise: warn — source/test edits outside an impl/fix state (current: the current state)

Check a path before writing it:

```bash
bash .agents/skills/vibe/scripts/detect-context.sh decide <path>
```

## Delegating to sub-agents

Model tiers for ANY delegated work — Agent-tool calls and Workflow-script `agent()`
calls alike. Set the `model` parameter explicitly on every call; never omit it
(omission silently inherits the session model):

- `haiku` — mechanical bulk work: renames, boilerplate, format conversion, log triage.
- `sonnet` — default for well-specified implementation, exploration and per-task review.
- `opus` — architecture, adversarial or final review, genuinely tricky work:
  concurrency, subtle algorithms, gnarly debugging.

When unsure between tiers, pick the cheaper and escalate on failure.

## Dynamic workflows (Workflow tool)

Applies to ALL sessions, any model. Dynamic workflows do not need to be avoided —
reach for the Workflow tool when a task has 3+ independent parallelizable subtasks or
would benefit from a pipeline/judge panel.

Opt-in: unless your human partner already asked for orchestration in their own words,
plan first — propose the workflow in one or two sentences with the rough shape and
cost, and wait for their reply; their "yes" is the opt-in.

**Agent models inside workflow scripts:** every `agent()` call MUST set the `model`
parameter explicitly, chosen per "Delegating to sub-agents" above. Only `haiku`,
`sonnet`, or `opus`.
<!-- /vibe:rules -->
