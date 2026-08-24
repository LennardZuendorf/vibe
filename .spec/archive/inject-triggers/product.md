---
type: feature-product
feature: inject-triggers
sibling: tech.md
parent: ../../product.md
updated: 2026-08-15
---

# Feature: Inject Triggers — Product

Finishes the injection arc that content-layer started. Three things change.
Injected payload is classed by **trigger**, so the transcript stops paying for
the same fifteen lines on every turn: a two-line level channel every turn, the
full orders only when the cursor moves, and event text only when an event
happened. The **write invariants become data** (`policy.json`), read by the
enforcer and the renderer alike, so the seven prose restatements stop being able
to disagree with the code. And the surfaces that carry stale or duplicate text —
the `SessionStart` cursor line, the unbounded warn relay, the stop gate's third
predicate — are corrected.

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `flow/content/policy.json`, trigger classing in `flow/engine/content.mjs`, `.vibe/last-inject`, the `user-prompt.{level,edge,event}` channels, `vibe policy` in the engine, `detect-context.sh`'s delegation to it, the doctrine block's invariant text, warn-relay bounds, stop predicate 3 |
| **Does not own** | The cursor writer and gate enforcement (machine-teeth); the plugin carrier (plugin-runtime); the spec validators (spec-js); the block/channel/override machinery itself (content-layer, delivered) |

---

## Requirements

### Requirement: Payload is classed by trigger (R1)

The system SHALL class each prompt channel as `level`, `edge`, or `event`, and
MUST emit an `edge` channel only when the cursor differs from the cursor at the
last inject.

#### Scenario: Two turns in the same state

- **Given** a cursor that does not change between two turns
- **When** the inject hook runs on the second turn
- **Then** the level channel is emitted again and the edge channel is not

#### Scenario: The turn after a transition

- **Given** a cursor that moved since the last inject
- **When** the inject hook runs
- **Then** the edge channel is emitted in full, and the recorded cursor is updated

### Requirement: The level channel is minimal and byte-stable (R2)

The `user-prompt.level` channel MUST be at most two lines and MUST name the
current state and its transition command.

#### Scenario: Prompt-cache stability

- **Given** two turns in the same state with no drift and no warnings
- **When** both injects are compared
- **Then** they are byte-identical

### Requirement: Write invariants are data (R3)

`flow/content/policy.json` SHALL be the single source for the write invariants,
and both the enforcer and the rendered prose MUST derive from it.

#### Scenario: One edit changes both

- **Given** a change to a rule's allowed states in `policy.json`
- **When** the guard decides a path and the doctrine renders
- **Then** both reflect the change with no second edit

#### Scenario: The enforcer degrades without node

- **Given** a target with no `node` on PATH
- **When** `detect-context.sh decide` runs against a guarded path
- **Then** it returns the same verdict as the engine would, from its own fallback

### Requirement: Live state never rides SessionStart (R4)

`SessionStart` output MUST NOT contain the cursor, because that output replays
stale after `--resume`.

#### Scenario: Resume

- **Given** a session resumed hours after the cursor moved
- **When** the `SessionStart` payload is replayed
- **Then** it names no state

### Requirement: The warn relay is bounded and deduplicated (R5)

The relay MUST collapse identical queued warnings and MUST cap what one turn
emits.

#### Scenario: A loop queues the same warning forty times

- **Given** forty identical warn lines in the relay log
- **When** the next inject drains it
- **Then** one line is emitted, with a count, and the log is truncated

### Requirement: The stop gate keeps only teeth that earn their place (R6)

Stop predicate 3 (the stuck-phase nudge) MUST be removed, because the level
channel now carries the same information every turn.

#### Scenario: A non-idle state with legal next states

- **Given** a cursor in `feature.impl` and a stop event
- **When** the gate runs
- **Then** no stuck-phase warning is queued
