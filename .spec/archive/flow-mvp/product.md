---
type: feature-product
feature: flow-mvp
sibling: tech.md
parent: ../../product.md
updated: 2026-07-07
---

# Feature: flow-mvp — the personal operating layer

## Problem

vibe's flow half names its delegates but does not contract with them, asks for
confirmation at every transition, and keeps every hook advisory. The result: the
two pressures vibe exists to remove — prompting each stage by hand, and policing
"done" claims — persist. Meanwhile the upstream skills' own conventions
(artifact paths, self-commits, chain handoffs) silently fight the flow at every
seam (see [research: part 2](../vibe-flow/research.md)).

This feature reworks the flow into the MVP of the reinterpreted vibe: **a
personal operating layer** — vibe decides *when* and *where* (stage advancement,
artifact destinations, memory), delegates decide *how* (methodology). Not a
product; a portable preference set for the owner's projects.

**Definition of done (owner-level):** (1) plans are written into `.spec/` and
executed from `.spec/` natively by superpowers' executors; (2) after "I need X",
the flow advances itself and consults the human only at the two gates.

## Scope

| Owns | Does not own | Deferred |
|---|---|---|
| `flow/` phase files (skill shims), `flow/SKILL.md` orders | `install.sh`, adapters, spec-skill logic | Any further Stop-gate teeth beyond R6 |
| `flow/state-machine.json` (gates data, quick.compound, delegate lists) | Re-sweep of this repo's stale `.spec/` root docs (separate quick task) | README/positioning rewrite (strategy.spec follow-up) |
| `.claude/hooks/stop-gate.sh` verify tooth + evidence convention | `pre-tool-use-guard.sh` Bash-hole (accepted as warn-first) | CE-style categorized lessons |
| `spec/reference/templates/feature-plan.md` + `spec/feature.md` step 5 (hybrid grammar, root-plan-row timing — deliberate cross-half touches) | spec skill subagents | wenyan and other caveman extras |
| `flow/reference/deps.json`, `doctor.sh` (caveman demotion) | | |
| `flow/tests/` (hermeticity + new assertions) | | |
| `.claude/commands/flow.md` (one line: drop stop-per-transition) | | |

## Requirements

### Requirement: R1 — Precedence contract

`flow/SKILL.md` and the AGENTS.md template block SHALL state: the cursor owns
sequencing and artifact destinations; delegated skills execute within the
current state's scope; their artifact-path conventions, self-commit steps, and
next-skill handoffs are overridden; transitions only via `set-state.sh`.

#### Scenario: upstream handoff ignored
- Given `feature.impl` with `superpowers:executing-plans` finishing its tasks
- When the upstream skill's terminal step names `finishing-a-development-branch`
- Then the agent transitions to `feature.verify` instead, per precedence.

### Requirement: R2 — Delegation contract blocks

Every delegation site in a phase file SHALL be a contract block naming: inputs
to inject (constraint sections, templates, paths), outputs to redirect (exact
`.spec` destination), upstream steps to skip, and a **non-blocking announce**
("delegating to `X` — say *self* to keep it inline") that proceeds without
waiting; self-suffice on decline or absence. `.spec/.config.yaml
suggest-superpowers: false` is a standing decline (mirrors the spec skill).
Redirect/skip lines MUST be copied into any subagent (Task) prompt — subagents
receive no per-turn orders. (Non-blocking per the lessons.md rule "never block
on the answer"; this closes the flow half's "promote proactively" gap without
re-introducing stage-prompting.)

#### Scenario: brainstorm seam
- Given `strategy.brainstorm` (write surface: none)
- When delegating `superpowers:brainstorming`
- Then the block scopes it to dialogue phases only, skipping the upstream
  design-doc write + self-commit; the artifact lands in the next state.

### Requirement: R3 — Hybrid plan grammar

The feature-plan template SHALL keep `{name}/n` stable units as canonical and
embed executor-parseable structure: a top `## Global Constraints` section and
per-unit checkbox **Steps** (writing-plans style). `feature.plan` SHALL redirect
`superpowers:writing-plans` to it via its documented storage-location seam.

#### Scenario: executor reads the spec plan
- Given a `plan.md` produced under the hybrid template
- When `superpowers:executing-plans` (or SDD) consumes it
- Then tasks resolve from unit Steps without reformatting, and unit IDs remain
  citable in commits/tests.

### Requirement: R4 — Auto-advance with two gates

Non-gated transitions SHALL advance automatically (`set-state.sh` + one-line
announcement, no confirmation question). The flow SHALL stop for approval only
at the two human gates, encoded as machine data on **edges** (not states):
`feature.plan>feature.impl` and `feature.verify>feature.compound` — the
verify→impl fix loop stays ungated. Exception: `quick.triage → feature.design`
escalation is announce-and-confirm (it renames the work and names a feature).
`set-state.sh idle` SHALL always be legal — abort ends any flow; `idle` joins
`next` of mid-arc states. Ambient instructions SHALL agree with this rule: the
AGENTS.md template's "Ask first" transition rows and the `/flow` command's
stop-per-transition line are rewritten to gated-edges-only.

#### Scenario: no stage-prompting
- Given `feature.design` exit criteria met
- When the design docs are written
- Then the agent calls `set-state.sh feature.plan` and continues without asking.

### Requirement: R5 — Two impl modes

`feature.impl` SHALL offer **interactive** (`superpowers:executing-plans`) and
**handover** (`superpowers:subagent-driven-development`) modes, both consuming
the hybrid `plan.md`. SDD runtime artifacts stay in `.superpowers/**`
(gitignored, runtime-not-memory); both modes exit to `feature.verify`. Both
modes run on the **feature branch established at the plan gate** (per
flow-mvp/11a; a worktree is offered only in handover mode, consent-first), so
verify and the receipt run against the same tree. Mode is chosen at the plan
gate; default interactive.

#### Scenario: full handover still audited
- Given `feature.impl` in handover mode
- When SDD completes all plan units
- Then the flow transitions to `feature.verify` and the evidence check + human
  gate still run.

### Requirement: R6 — Verify tooth (first promoted Stop predicate)

In `*.verify` states the Stop hook SHALL block (exit 2) when no fresh evidence
receipt exists, and pass once one does. The receipt is a file the verify
procedure writes (commands + observed output per unit); names are fixed:
`evidence/feature-<feature>.md`, `evidence/quick.md`. Staleness is
**git-derived** (any changed file per `git status --porcelain` newer than the
receipt → stale; without git, existence-only). The hook SHALL pass through when
`stop_hook_active` is set or the cursor is unreadable. The block message's
escape hatch is `set-state.sh idle` (abort) — never "write the receipt by
hand". All other Stop predicates remain warn-only.

#### Scenario: done-claim without evidence
- Given `feature.verify` with source newer than any receipt
- When the session tries to stop
- Then the Stop hook blocks with instructions to run verification and write the
  receipt.

### Requirement: R7 — Quick flow can compound

The quick flow SHALL gain a lessons path: `quick.verify → quick.compound
(optional) → idle`, where `quick.compound` may append a tagged lesson and
regenerate the digest. Skippable — most quick fixes surface no durable lesson.

#### Scenario: quick fix surfaces a lesson
- Given `quick.verify` passing with a durable lesson identified
- When the agent enters `quick.compound`
- Then the lessons.md append is allowed by the write policy and the digest
  regenerates.

### Requirement: R8 — Caveman demoted to vocabulary

The `caveman` entry SHALL be removed from `deps.json` (and doctor's dep rows).
The three levels remain vibe vocabulary, frozen per state, with an attribution
note. No behavioral change to orders.

#### Scenario: doctor after demotion
- Given a repo with no caveman skill installed
- When `doctor.sh` runs
- Then no `dep.caveman` row appears and exit stays 0, while
  `check-skills.sh caveman full` still prints the frozen level definition.

### Requirement: R9 — Single router at idle

`idle` SHALL NOT delegate `superpowers:using-superpowers`. Skill discovery
remains superpowers' own concern; vibe's orders are the router.

#### Scenario: one router
- Given a fresh session at `idle`
- When the inject hook fires
- Then the orders name only vibe's four flows and no competing meta-router
  skill.

### Requirement: R10 — Tests protect the rework

The flow test suite SHALL be hermetic (no reads/writes of the live repo cursor)
and SHALL assert: machine `delegates` ⊆ names mentioned in the linked phase
file; gate data matches prose; evidence-gate blocks and passes; every orders
block stays ≤ 400 bytes.

#### Scenario: concurrent suites
- Given two `flow/tests/run.sh` invocations running simultaneously
- When both complete
- Then both exit 0 and a pre-existing live cursor is byte-identical.

## Non-goals

- Enforcing flow legality in `set-state.sh` (writer-not-gate stands).
- Closing the PreToolUse Bash hole (accepted as a seatbelt, re-worded not
  re-armed).
- Replacing or vendoring any superpowers/feature-dev content.

## Resolved questions

1. `spec/feature.md` step 5 vs write policy (root plan row blocked in
   `feature.plan`): **owned by flow-mvp/10** — step 5 is edited so the root
   Feature Sequence row is added at the `feature.compound` promote step. The
   flow-mvp row itself lands then.
