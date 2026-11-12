# AGENTS.md — vibe Engineering Guide

<!-- vibe:instructions:start -->
<!-- Managed by vibe (merge-agents.sh); replaced on setup.apply / install.
     Content outside these markers, including vibe:active-rules, is user-owned. -->
This project uses **Vibe Flow**. `vibe` explains the toolkit, `flow` owns the lifecycle,
and `spec` handles project knowledge. Direct Flow and Spec entry need no bootstrap.
## Session start
1. Read the spec root's `lessons.md` and `plan.md` when present. A flow-only install has no spec tree.
2. Read the active design and plan in `docs/superpowers/`; fallback: `docs/spec/features/<name>/`.
3. Read `.vibe/run/flow/state.json` first; otherwise use the supported legacy cursor. Only absence means idle. Adopted v2 state never falls back to legacy after corruption or removal. Load `flow` and its current orders; report corrupt state for explicit recovery.
## Working model
`design → plan →[gate] impl → compound → verify →[gate] ship`
Get explicit approval before crossing: plan → impl, and verify → ship.
Freeze approved design/plan; keep progress in runtime records. Workers receive
bounded scopes/checks and return results; the controller owns state and independent review.
Compound prepares additive docs and lessons before final checks. Preserve feature artifacts.
Never infer permission to commit, push, merge, release, delete artifacts or clean up.
## Runtime and policy
Gates are a convention today for legacy v1: `set-state.sh` writes the cursor; it does not gate.
Explicit v2 adoption enables legal-edge/revision checks and immutable records.
Use `vibe-flow state transition` for adopted runs; legacy writers refuse them.
Production trusted approval/evidence checks remain unavailable/deny. A `confirm`
token, hook registration or model-written receipt proves neither human origin nor readiness.
Follow current-state orders from `flow`; delegates supply bounded methods.
Never hand-edit state. Explicit abort is not completion or ship authorization.
Restricted paths are data in `content/policy.json`; inspect `detect-context.sh decide` before writes.
Edit blocks/config, not generated regions. Standing prompt/rules channels use root `vibe.json`.
## Commands
```bash
vibe-spec validate # before claiming done; absent on --only flow
bash .agents/skills/flow/scripts/detect-context.sh decide <path>
bash .agents/skills/flow/scripts/doctor.sh
node .agents/skills/flow/engine/cli.mjs render --check
node .agents/skills/flow/engine/cli.mjs render agents-md --write
```
## Enforcement limits
Legacy edit guards and receipt freshness do not cover every shell, interactive
input or subagent action. Native complete blocking is unverified for Claude Code,
OpenCode and Codex. Strong checks stay unavailable/deny; configured hooks are not proof.
Task results and command-output metadata remain claims until independently checked.
Candidate changes require fresh final verification/review and approval of the named action.
**Output: brief technical English** — answer first, one idea per sentence. Keep security warnings,
irreversible-action confirmations, code, paths and commands byte-exact.
<!-- vibe:instructions:end -->

<!-- vibe:active-rules:start -->
<!-- Generated from lessons.md by regen-active-rules.sh. Do not edit by hand. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->

<!-- vibe:rules -->
_Managed by vibe — rendered from the `agents-md` channel by `node .agents/skills/flow/engine/cli.mjs render agents-md --write`. Edit the blocks or `vibe.json`, not this region._

## Write invariants

Every write this harness restricts is one rule in `content/policy.json`. The
list below is RENDERED from that file — the same data
`detect-context.sh decide` enforces — so the prose and the enforcer cannot
disagree. Edit the rules, never this text.

- `.vibe/run/flow/state.json`, `.agents/skills/flow/state.json`, `.agents/skills/vibe/state.json`, `.agents/flow/state.json` — otherwise: block — state.json is written only via set-state.sh, never by direct edit
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
bash .agents/skills/flow/scripts/detect-context.sh decide <path>
```

## Delegating to sub-agents

Choose the needed capability before its provider or model. Apply controlling user
and repository model preferences. Set an allowed available model explicitly for
all delegated work; do not infer one from the role name. Report unavailable
required capabilities instead of silently substituting a forbidden model.

Workers receive exact scope, dependencies and checks. The controller owns state,
progress, gate decisions and independent review. A worker returns checked-result
claims; it cannot widen scope, approve itself, commit, clean or finish a branch.

## Coordinating independent work

Use the host's available orchestration only when authorized by the controlling
instructions. If orchestration requires opt-in, propose its scope and cost and
wait for the user's reply. A workflow tool is optional; it does not replace the
controller, authorization gates or independent review.

Parallelize independent work on disjoint files when permitted. Serialize shared
files and dependent tasks. Supply the same bounded TaskBrief and TaskResult
contract to native named agents or generic agents. Preserve controlling model
preferences on every dispatch. Missing required independent review blocks readiness.
<!-- /vibe:rules -->
