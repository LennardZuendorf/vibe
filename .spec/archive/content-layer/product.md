---
type: feature-product
feature: content-layer
sibling: tech.md
parent: ../../product.md
updated: 2026-08-15
---

# Feature: Content Layer — Product

Every rule vibe injects into a session — per-turn prompt text, session-start
doctrine, and the standing rules in `AGENTS.md` — becomes an authored **block**
composed into a named **channel**, configured as data. A project turns any rule
on or off, reorders it, rewrites it, or adds its own from a single root
`vibe.json`, without editing anything vibe ships and without losing its changes
on upgrade. Ships with three default rulesets: brief technical English (per
turn), sub-agent model tiers, and dynamic-workflow usage (both `AGENTS.md`).

**Parent:** [../../product.md](../../product.md)
**Architecture:** [tech.md](tech.md)
**Plan:** [plan.md](plan.md)

---

## Scope

| | |
|---|---|
| **Owns** | `flow/content/**` (shipped config + block library), `flow/engine/content.mjs`, `flow/engine/commands/render.mjs`, the `vibe:rules` managed block in `AGENTS.md`, the root `vibe.json` contract, the write half of the marker grammar in `flow/engine/blocks.mjs` |
| **Does not own** | The flow orders (`vibe:orders:<state>` blocks in `flow/SKILL.md`) and the doctrine block — this feature composes AROUND them, it does not migrate them; the write invariants (`detect-context.sh decide`); the cursor; trigger classing (level/edge/event), which is inject-triggers' box |

---

## Requirements

### Requirement: Content is authored once, as data (R1)

The system SHALL resolve every injected rule from a block library plus JSON
configuration, and MUST let a project override any shipped channel or block from
a root `vibe.json` that vibe never rewrites.

#### Scenario: Project silences a shipped rule

- **Given** a shipped block composed into the per-turn channel
- **When** the project sets that block's `enabled` to `false` in `vibe.json`
- **Then** the channel renders without it, and no error is reported

#### Scenario: Project adds its own rule

- **Given** a `vibe.json` that defines a block inline (or points at a file) and adds its id to a channel
- **When** that channel renders
- **Then** the project's rule is composed in the configured order

### Requirement: One block, two verbosities, many channels (R2)

A block MUST carry a terse `summary` for prompt channels and a prose `body` for
document channels, and the system SHALL render the form the channel declares.

#### Scenario: The same authored rule serves both surfaces

- **Given** a block with a summary and a body
- **When** it is composed into a `summary` channel and a `body` channel
- **Then** the terse form and the prose form are rendered respectively, from one source

### Requirement: Machine fields interpolate as typed placeholders (R3)

The system SHALL interpolate `{{state}}`, `{{feature}}`, `{{next}}`, `{{writes}}`,
`{{reads}}`, `{{delegates}}`, `{{exit}}`, `{{orders}}`, `{{doctrine}}`,
`{{lessons:TAG}}` and project-defined names, and MUST leave an unknown
placeholder literally in place while reporting it.

#### Scenario: A typo is visible, not blank

- **Given** a block referencing `{{nope}}`
- **When** the channel renders
- **Then** the text still shows `{{nope}}` and the lint reports an unresolved placeholder

### Requirement: Injection fails shut, never fatal (R4)

A missing, unreadable, or malformed content layer MUST reduce the injected text
and MUST NOT fail a hook, a turn, or a session.

#### Scenario: Broken project config

- **Given** a `vibe.json` that is not valid JSON
- **When** a hook renders its channel
- **Then** the shipped defaults still render, the error is reported by the lint, and the hook exits 0

### Requirement: Budgets and lints are enforced by a command (R5)

The system MUST provide a check that fails on unknown block ids, exceeded
per-channel line budgets, unresolved placeholders, and an empty content tree.

#### Scenario: A check that examined nothing fails

- **Given** a target with no content tree at all
- **When** the check runs
- **Then** it exits non-zero rather than reporting a clean pass

### Requirement: Document channels sync into a managed block (R6)

The system SHALL render a document channel into a marker-bounded block in its
configured file, MUST leave all content outside those markers untouched, and MUST
NOT rewrite the file when the rendered content is unchanged.

#### Scenario: AGENTS.md sync

- **Given** an `AGENTS.md` holding user prose
- **When** `vibe render agents-md --write` runs twice
- **Then** the first run writes the `vibe:rules` block, the second reports no change, and the user prose survives both

### Requirement: Three default rulesets ship enabled (R7)

The shipped defaults MUST compose a brief-technical-English rule into the
per-turn channel, and sub-agent model tiers plus dynamic-workflow usage into the
`AGENTS.md` channel.

#### Scenario: Fresh install

- **Given** a fresh `install.sh` run on a target with node available
- **When** the install completes
- **Then** `AGENTS.md` carries the delegation and workflow rulesets, and the per-turn hook injects the style rule
