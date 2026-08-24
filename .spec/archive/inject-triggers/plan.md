---
type: feature-plan
feature: inject-triggers
sibling: tech.md
parent: ../../plan.md
updated: 2026-08-15
---

# Feature: Inject Triggers — Implementation Plan

Six units, executed by subagent-driven development. Units 1–2 make the write
invariants data; units 3–4 class the prompt payload by trigger; unit 5 removes
the surfaces the first four make redundant; unit 6 is docs and the evidence
receipt.

**Parent:** [../../plan.md](../../plan.md)
**Requirements:** [product.md](product.md)
**Architecture:** [tech.md](tech.md)

**Feature gate:** Starts when content-layer is `DONE` (delivered on this branch — `flow/engine/content.mjs`, `flow/content/**`, suites green).

---

## Problem Frame

content-layer made injected content configurable but left three defects the
2026-08 review named. Every turn pays for the full orders even when nothing
moved. The write invariants are prose in seven places and code in one, kept in
step by a test that can only notice disagreement after the fact. And three
surfaces carry text that is stale (`SessionStart`'s cursor line), unbounded (the
warn relay), or duplicated (stop predicate 3 restates what the orders already
say). This plan closes all three.

---

## Requirements Trace

| ID | Requirement | Units |
|---|---|---|
| R1 | [Payload is classed by trigger](product.md#requirement-payload-is-classed-by-trigger-r1) | inject-triggers/3, inject-triggers/4 |
| R2 | [The level channel is minimal and byte-stable](product.md#requirement-the-level-channel-is-minimal-and-byte-stable-r2) | inject-triggers/4 |
| R3 | [Write invariants are data](product.md#requirement-write-invariants-are-data-r3) | inject-triggers/1, inject-triggers/2 |
| R4 | [Live state never rides SessionStart](product.md#requirement-live-state-never-rides-sessionstart-r4) | inject-triggers/5 |
| R5 | [The warn relay is bounded and deduplicated](product.md#requirement-the-warn-relay-is-bounded-and-deduplicated-r5) | inject-triggers/5 |
| R6 | [The stop gate keeps only teeth that earn their place](product.md#requirement-the-stop-gate-keeps-only-teeth-that-earn-their-place-r6) | inject-triggers/5 |

---

## Key Technical Decisions

1. **The bash fallback stays.** `detect-context.sh decide` delegates to the
   engine when node exists and answers from its own branch when it does not. The
   guard is a hard block; losing it to a missing runtime is not acceptable.
2. **A differential test replaces the prose parity test, and only after it
   passes.** Never delete an existing guard in the same unit that adds its
   replacement without the replacement proving itself first.
3. **Edge detection fails open.** A missing or unreadable `.vibe/last-inject`
   means "treat this turn as an edge" — a first turn after install must carry the
   full orders.
4. **No behaviour change without a test that fails first.** Every unit here
   touches a hook path; a regression is invisible until a session breaks.

---

## Global Constraints

<!-- Executors: read this section before starting any unit. -->

- Zero runtime dependencies. Node 18+. Bash MUST be `set -euo pipefail` and shellcheck-clean.
- No engine module may name `.spec`, `.agents/skills/vibe`, `state.json`, `state-machine.json`, `CLAUDE_PROJECT_DIR`, or the `<!--` marker grammar — `flow/engine/tests/primitives.test.mjs` enforces this with per-line, occurrence-counted waivers. Route through the owning primitive instead.
- Every hook path MUST NOT throw. Degrade to less output, never to a failed hook.
- Before claiming a unit done: `bash tests/run.sh` (all four suites), `bash .agents/skills/spec/scripts/validate.sh`, `bash spec/scripts/check-drift.sh`, `git ls-files '*.sh' | xargs shellcheck`, and `node flow/engine/cli.mjs render --check`.
- Run the no-jq leg for any change touching bash: strip jq from PATH and re-run `bash tests/run.sh`.
- Cite the unit ID in every commit subject: `feat(flow): inject-triggers/N …`.
- Conventional Commits, imperative, lowercase, ≤50-char subject.
- Never edit `.agents/skills/vibe/state.json` by hand.

---

## Unit IDs

Units are `inject-triggers/n`, assigned once and never renumbered.

---

### inject-triggers/1 — `policy.json` + the policy engine

**Goal:** the three write invariants become data, with an engine that decides a path against a state.

**Requirements:** R3

**Dependencies:** —

**Files:**

```
flow/content/policy.json            # rules: id, match, states, verdict, reason
flow/engine/policy.mjs              # loadPolicy/decide/renderInvariants
flow/engine/commands/policy.mjs     # vibe policy decide <path> [state] | list | render
flow/engine/cli.mjs                 # + 'policy' in COMMANDS
flow/engine/tests/policy.test.mjs   # new
flow/engine/tests/run.mjs           # placeholder-CLI helper marker (COMMANDS literal)
flow/engine/tests/cli.test.mjs      # COMMANDS list
```

**Interfaces:**
- Produces `loadPolicy(vibeDir)`, `decide(policy, relPath, state)`, `renderInvariants(policy)`.
- `decide` returns `{verdict, reason, ruleId}`; `verdict` is `allow` | `warn` | `block`.
- Rule matching is exact-path first, then glob; the first matching rule wins.

**Test scenarios:**
- Each of the three shipped rules blocks outside its states and allows inside them.
- An unmatched path is `allow`.
- A malformed or absent `policy.json` degrades to `allow` for everything, and reports.
- Prototype keys (`__proto__`, `constructor`) as rule ids or paths resolve nothing.
- `vibe policy decide` exit codes: 0 allow, 0 warn, 0 block — the verdict is stdout, never the exit code (the hook translates it).

**Verification:** `node flow/engine/tests/run.mjs policy` green; `node flow/engine/cli.mjs policy list` prints three rules.

---

### inject-triggers/2 — delegation + generated invariant prose

**Goal:** the enforcer and every prose surface derive from `policy.json`.

**Requirements:** R3

**Dependencies:** inject-triggers/1

**Files:**

```
flow/scripts/detect-context.sh      # decide delegates to `vibe policy decide` when node exists
flow/engine/content.mjs             # + {{invariants}} placeholder
flow/content/blocks/flow/invariants.md
flow/engine/tests/policy.test.mjs   # + the differential matrix
flow/tests/run.sh                   # the prose parity test is REPLACED here, not before
```

**Interfaces:**
- Consumes `decide()` from unit 1.
- `{{invariants}}` renders one line per rule: path, allowed states, reason.

**Test scenarios:**
- Differential matrix: every guarded path × all 13 states, decided through the engine and through the bash fallback, verdicts equal.
- `detect-context.sh decide` keeps its exact exit codes and stdout with node present AND with node stripped from PATH.
- `{{invariants}}` matches what the doctrine block states today (assert on the rule set, not on a hand-copied sentence).
- The replaced prose-parity test is gone only after the differential test passes; the suite count does not silently drop.

**Verification:** `bash tests/run.sh`; the same run with node removed from PATH; `bash flow/tests/run.sh` green.

---

### inject-triggers/3 — trigger classing + edge detection

**Goal:** channels declare a trigger; the engine records and compares the cursor between injects.

**Requirements:** R1

**Dependencies:** inject-triggers/1

**Files:**

```
flow/engine/content.mjs             # channelTrigger(), cursorChangedSince(), recordInject()
flow/engine/tests/content.test.mjs  # + trigger cases
.gitignore                          # .vibe/last-inject
```

**Interfaces:**
- Produces `channelTrigger(channel)`, `cursorChangedSince(root, key)`, `recordInject(root, key)`.
- The recorded key is the cursor state plus feature, one line, written atomically.

**Test scenarios:**
- Same cursor twice → `cursorChangedSince` false on the second call.
- Cursor moved → true, and the recorded key updates.
- Missing/unreadable/corrupt `.vibe/last-inject` → true (fail open), never throws.
- An unwritable `.vibe/` directory → still true, still no throw.
- A channel with no declared trigger defaults to `level`.

**Verification:** `node flow/engine/tests/run.mjs content` green.

---

### inject-triggers/4 — the three prompt channels

**Goal:** the inject hook emits level every turn, edge on a cursor change, event only on events.

**Requirements:** R1, R2

**Dependencies:** inject-triggers/3

**Files:**

```
flow/content/vibe.default.json      # user-prompt.level / .edge / .event channels + budgets
flow/content/blocks/flow/level.md   # {{state}} + its transition command, ≤2 lines
flow/content/blocks/flow/edge.md    # {{orders}}, {{delegates}}, {{lessons:<state>}}
flow/engine/commands/hook.mjs       # compose by trigger; record the inject
flow/engine/tests/content.test.mjs  # + channel composition cases
flow/engine/tests/hook.test.mjs     # + per-turn payload cases
```

**Interfaces:**
- Consumes `channelTrigger`/`cursorChangedSince`/`recordInject` from unit 3.
- Emission order stays: drift → level → edge (when due) → other prompt channels → warns.
- The legacy `user-prompt` channel keeps working; a project that never configures the new channels sees no change.

**Test scenarios:**
- Two turns, same state, no drift, no warns → byte-identical stdout, and it is ≤2 lines plus the standing rules.
- Turn after `set-state.sh` → the edge channel appears exactly once, then not again.
- Budgets: level ≤2 lines, edge ≤15 — over budget is an error from `render --check`.
- No content tree → the hook output is unchanged from today.

**Verification:** `node flow/engine/tests/run.mjs` green; `bash flow/tests/adapters/run.sh` green.

---

### inject-triggers/5 — the subtractions

**Goal:** remove the stale cursor line, bound the relay, delete predicate 3.

**Requirements:** R4, R5, R6

**Dependencies:** inject-triggers/4

**Files:**

```
flow/engine/commands/doctrine.mjs   # drop the cursor summary line
flow/scripts/doctrine.sh            # same, so the bash oracle still matches
flow/engine/commands/hook.mjs       # relay: dedupe + cap at 10 lines; delete predicate 3
flow/engine/tests/doctrine.test.mjs # updated expectations
flow/engine/tests/hook.test.mjs     # relay bounds; predicate 3 absence
flow/tests/run.sh                   # any assertion naming predicate 3
```

**Test scenarios:**
- `SessionStart` payload contains no state name, in either the engine or the bash path.
- 40 identical warn lines → one emitted line carrying `(x40)`; log truncated.
- 50 distinct warn lines → 10 emitted, with a "+40 more" trailer; log truncated.
- A non-idle cursor at stop → no stuck-phase warning queued.
- The evidence-receipt tooth (predicate 2) still blocks — deleting predicate 3 must not touch it.

**Verification:** `bash tests/run.sh`; `bash flow/tests/adapters/run.sh`; the no-jq leg.

---

### inject-triggers/6 — docs, config surface, evidence

**Goal:** the READMEs and the AGENTS.md template describe the trigger classes and `policy.json`; the receipt is written.

**Requirements:** R1, R3

**Dependencies:** inject-triggers/1–5

**Files:**

```
flow/README.md                                  # trigger table + policy.json
README.md                                       # one paragraph
flow/reference/templates/AGENTS.md              # invariants sourced from the render
AGENTS.md                                       # re-rendered managed blocks
.agents/skills/vibe/evidence/feature-inject-triggers.md
```

**Test scenarios:**
- `node flow/engine/cli.mjs render agents-md --write` is a no-op after the docs land (idempotence).
- `bash spec/scripts/check-drift.sh` clean.

**Verification:** all four suites green; `validate.sh` 0 errors; receipt written with quoted command output.
