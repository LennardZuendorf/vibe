---
type: feature-product
feature: flow-legibility
sibling: tech.md
parent: ../../product.md
updated: 2026-07-18
---

# Feature: flow-legibility — Product

flow-legibility makes the vibe flow self-explaining and self-driving without
adding hard rails. It closes the five legibility gaps found in the 2026-07 rework
(`docs/brainstorms/2026-07-17-vibe-rework.md` §5–6, cross-cutting): the per-turn
inject states the transition duty *imperatively* instead of naming a label; a
`SessionStart` hook delivers the working-model doctrine every session so
uncontrolled repos need no `AGENTS.md`; the state machine gains the missing
loop/back edges; cursor-drift is inferred from working-tree activity and surfaced
first; and every subagent dispatch names a model tier. It is the first rework
feature and unblocks delegation-redirect and vibe-plugin.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)

---

## Scope

| | |
|---|---|
| **Owns** | Imperative self-carrying orders in `flow/SKILL.md` `vibe:orders:*` blocks + the `idle` inject in `state-machine.json`; a `<!-- vibe:doctrine -->` block + `flow/scripts/doctrine.sh` resolver; `.claude/hooks/session-start-doctrine.sh` and its `merge-settings.sh` / `settings.json` `SessionStart` wiring; the machine edges `strategy.spec→strategy.brainstorm` and `feature.plan→feature.design`; `research.md` as a first-class `feature.design` write; activity→state drift inference in `detect-context.sh` + drift-first placement in `user-prompt-submit-inject.sh`; model-tier pins in every delegation contract block (`flow/*.md`); a `doctor.sh` instruction-coverage check; matching `flow/tests/run.sh` + `flow/tests/adapters/run.sh` assertions. |
| **Does not own** | The `PostToolUse`/`Skill` redirect hook + `redirects.json` and the superpowers-native plan format (→ delegation-redirect). Per-user plugin packaging and `install.sh --local` team-repo mode (→ vibe-plugin); this feature ships the `SessionStart` hook + in-repo `settings.json` wiring only. The bash spec delta engine (→ spec-delta). Stop-hook mechanical auto-advance (deferred — agent-run only). |

---

## Requirements

### Requirement: Self-carrying orders (R1)

The per-turn inject SHALL state the transition duty imperatively — naming the
`set-state.sh <next>` command (or, at a gated edge, the `/flow <next> confirm`
gate) — in every state's orders block, so an agent that reads only the inject
knows how to advance without consulting phase-file prose. Orders blocks MUST stay
≤400 bytes, byte-stable, and byte-identical under jq and no-jq.

#### Scenario: Non-gated edge names its command

- **Given** the cursor is at `feature.design`
- **When** the inject is produced for the turn
- **Then** the orders block directs advancing to `feature.plan` via `set-state.sh` (not merely `next: feature.plan`)

#### Scenario: Gated edge names the gate, not a bare transition

- **Given** the cursor is at `feature.plan` (a gated source edge)
- **When** the inject is produced
- **Then** the block directs the human-gated `/flow feature.impl confirm` and retains its `gate:` marker

#### Scenario: Byte budget and parity preserved

- **Given** any state's orders block
- **When** resolved via `orders.sh` under jq and under a no-jq PATH
- **Then** the block is ≤400 bytes and byte-identical between the two

### Requirement: SessionStart doctrine hook (R2)

The system SHALL deliver the working-model doctrine (session-start reads,
durable-vs-ephemeral framing, the write invariants, the two human gates, the
"you drive the flow" contract, and a current-cursor summary) through a
`SessionStart` hook each session and re-inject it after compaction, sourced from
a single canonical doctrine block shared with the `AGENTS.md` template, so a repo
with no `AGENTS.md` managed block still receives the doctrine.

#### Scenario: Doctrine injected at session start

- **Given** a vibe-enabled repo
- **When** a session starts
- **Then** the doctrine (working model + write invariants + gates + cursor summary) is injected as context

#### Scenario: Re-inject after compaction

- **Given** a session that has been compacted
- **When** the `compact` source fires
- **Then** the doctrine is re-injected

#### Scenario: Single source, no drift

- **Given** the doctrine text
- **When** it is emitted by both the hook and the `AGENTS.md` template
- **Then** both derive from the same `<!-- vibe:doctrine -->` block, never a duplicated copy

#### Scenario: Uncontrolled repo still covered

- **Given** a repo without the `AGENTS.md` `vibe:instructions` block but with the `SessionStart` hook wired
- **When** a session starts
- **Then** the doctrine is still delivered

### Requirement: Loop edges (R3)

The state machine SHALL permit iteration without abort-and-reenter by adding the
back-edges `strategy.spec → strategy.brainstorm` and `feature.plan →
feature.design`, and SHALL treat `research.md` as a first-class optional artifact
of `feature.design` — no new state or phase.

#### Scenario: Spec can return to brainstorm

- **Given** the cursor is at `strategy.spec`
- **When** direction needs re-shaping
- **Then** `set-state.sh strategy.brainstorm` is a legal transition

#### Scenario: Plan can return to design

- **Given** the cursor is at `feature.plan`
- **When** planning reveals a design gap
- **Then** `set-state.sh feature.design` is a legal transition

#### Scenario: Research stays in design

- **Given** the cursor is at `feature.design`
- **When** a discovery pass is needed
- **Then** `research.md` is an allowed write under `feature.design` and no state change is required

### Requirement: Drift-first nudges (R4)

The system SHALL infer a likely flow state from working-tree activity (e.g.
`src/` or `tests/` edits while the cursor is `idle` or otherwise outside an
impl/fix state) and surface a one-command correction as the FIRST line of the
next inject when drift is detected; when no drift is detected the inject stays
byte-stable (orders block unchanged, warnings still trailing). Drift nudges are
warn-only — they never block.

#### Scenario: Src edit while idle suggests a state

- **Given** the cursor is `idle`
- **When** `src/` files carry uncommitted changes
- **Then** the next inject's first line nudges `run /flow feature.impl or quick.fix` with the inferred target

#### Scenario: No drift keeps the inject stable

- **Given** activity consistent with the current cursor
- **When** the inject is produced
- **Then** no drift line is prepended and the orders block is byte-stable

### Requirement: Model-tier pins (R5)

Every delegation contract block and orders line that dispatches a subagent SHALL
name an explicit model tier — mechanical/exploration → sonnet;
review/architecture/synthesis → opus — so no subagent runs on an inherited
default.

#### Scenario: Explorer pinned to sonnet

- **Given** the `feature.design` `code-explorer` dispatch
- **When** its contract block is read
- **Then** it names sonnet

#### Scenario: Architect and reviewer pinned to opus

- **Given** a `code-architect` or `code-reviewer` dispatch
- **When** its contract block is read
- **Then** it names opus

### Requirement: Enforcement pattern preserved (R6)

The changes MUST keep the existing invariants: warn-first hooks (only the three
`detect-context.sh` hard blocks deny), graceful degradation (exit 0 on any
missing keystone; jq optional), machine⊆prose delegate parity, gate↔orders
consistency, and no-clobber marker merges.

#### Scenario: Graceful degrade holds

- **Given** jq is unavailable
- **When** the inject, doctrine, and drift inference run
- **Then** they degrade to warn/skip and never hard-fail

#### Scenario: New hook wiring is idempotent and non-clobbering

- **Given** an existing `.claude/settings.json` with user content
- **When** `merge-settings.sh` adds the `SessionStart` block
- **Then** existing hooks and user keys are preserved and re-running is a no-op

Reference R1–R6 in [plan.md](plan.md)'s Requirements Trace.

---

## Non-Goals

- **No new hard rails.** The fix is legibility and agency, not more blocking —
  drift nudges and self-carrying orders stay warn-first.
- **No Stop-hook auto-advance.** Orders are agent-run this feature; mechanical
  auto-advance on checkable edges is deferred.
- **No plugin or team-repo delivery.** The `SessionStart` hook ships via in-repo
  `settings.json` here; the per-user plugin and `--local` mode are vibe-plugin.
- **No skill-redirect hook.** The `PostToolUse`/`Skill` redirect is
  delegation-redirect.
