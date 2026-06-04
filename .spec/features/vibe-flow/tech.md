---
type: feature-tech
feature: vibe-flow
sibling: product.md
parent: ../../tech.md
updated: 2026-06-02
---

# Feature: Vibe Flow — Architecture

The vibe flow is a platform-neutral runtime layer under `.agents/flow` plus a
family of `vibe-*` agent skills. Its job is to carry the planning load: the user
says "I need X" and the flow constrains the agent into the right phase, with the
right skill, the right output path, and the right communication density already
chosen for it.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)

---

## Files

```text
.agents/
├── flow/
│   ├── state-machine.json
│   ├── state.example.json
│   └── scripts/
│       ├── detect-context.sh
│       ├── set-state.sh
│       └── validate-state.sh
└── skills/
    ├── vibe-strategy/SKILL.md
    ├── vibe-feature/SKILL.md
    ├── vibe-quick/SKILL.md
    ├── vibe-verify/SKILL.md
    ├── vibe-compound/SKILL.md
    ├── vibe-amend/SKILL.md
    └── vibe-setup/SKILL.md
```

---

## State Machine

States are compound keys of the form `<flow>.<phase>`. This is deliberate: a bare
`phase` cannot tell `feature.verify` from `quick.verify`, so transitions and the
per-state `next` array are keyed on the compound state, not on `phase` alone.

The mutable cursor in `state.json` carries only the moving parts:

```json
{
  "flow": "idle | setup | strategy | feature | quick",
  "phase": "idle | detect | apply | brainstorm | spec | design | plan | impl | verify | compound | triage | fix",
  "feature": null,
  "updated": "2026-06-02T00:00:00Z"
}
```

There is **no `notes` field**. Anything that varies turn-to-turn stays out of the
cursor so the per-turn inject can be cache-stable (see *Prompt Injection*). The
`updated` timestamp lives in the cursor only, never in injected text.

`state-machine.json` defines each state as a static entry. `caveman` and `inject`
are static per state — never stored in the mutable cursor — which is what keeps
the per-turn inject byte-stable:

```json
{
  "feature.impl": {
    "skill": "vibe-feature",
    "delegates": ["superpowers:executing-plans", "superpowers:test-driven-development"],
    "caveman": "full",
    "reads": [".spec/features/<feature>/plan.md"],
    "writes": ["src/**", "tests/**"],
    "inject": "skill=executing-plans+TDD · WRITE src/**, tests/** · do NOT edit .spec/** · cite plan unit IDs · caveman=full · next: verify",
    "next": ["feature.verify"],
    "exit": "tests reference plan unit IDs and pass"
  }
}
```

`set-state.sh` is the only sanctioned writer of `state.json`. Adapters and the
`/flow` command call it; they do not write the cursor directly.

### States, skills, and artifacts

| State (`flow.phase`) | superpowers | feature-dev agent | caveman | Spec artifact (R/W) | next |
|---|---|---|---|---|---|
| `idle` | `using-superpowers` | — | — | reads `lessons.md`, `plan.md` | setup.detect, strategy.brainstorm, feature.design, quick.triage |
| `setup.detect` | — | — | lite | reads repo | setup.apply |
| `setup.apply` | `writing-skills` | — | lite | **W** `.agents/**`, baseline `.spec/**` | idle |
| `strategy.brainstorm` | `brainstorming` | — | lite | **R** `lessons.md`; scratch | strategy.spec |
| `strategy.spec` | — | — | lite | **W** `.spec/{product,tech,design,plan}.md` | idle |
| `feature.design` | `brainstorming` | `code-explorer`, `code-architect` | lite | **R** `lessons.md`; **W** `features/<f>/{product,tech}.md` | feature.plan |
| `feature.plan` | `writing-plans` | `code-architect` | lite | **W** `features/<f>/plan.md` (stable unit IDs) | feature.impl |
| `feature.impl` | `executing-plans` + `test-driven-development` | — | full | **W** `src/**`, `tests/**` (cite unit IDs) | feature.verify |
| `feature.verify` | `verification-before-completion`, `requesting-code-review` (+`systematic-debugging` on fail) | `code-reviewer` | full | evidence | feature.compound, feature.plan, feature.impl |
| `feature.compound` | `finishing-a-development-branch` | — | lite (receipts ultra) | **W** tagged `lessons.md`, promote root specs, archive feature | idle |
| `quick.triage` | `systematic-debugging` | — | full | **R** `lessons.md` | quick.fix |
| `quick.fix` | `test-driven-development` | — | full | **W** `src/**`; opt `.spec/quick/<slug>.md` | quick.verify |
| `quick.verify` | `verification-before-completion` | `code-reviewer` | full | evidence | idle |
| `amend` (modifier) | `receiving-code-review` | — | lite | **W** targeted spec edits only | returns to prior state |

Three deliberate calls:

- **`quick.triage` is `full`, not `ultra`.** `ultra` compresses hardest and can drop
  edge cases; triage is exactly where a missed edge case is expensive. `ultra` is
  reserved for `feature.compound` receipts and subagent→orchestrator summaries.
- **`amend` is a modifier, not a flow.** It edits scope from any active state and
  returns there. The inject for `amend` carries the *target* state's write rules,
  not a separate amend block, to avoid colliding instructions.
- **Failure routes back.** `feature.verify` can return to `feature.plan` or
  `feature.impl`, which resolves the failed-verification routing the plan left open.

### Stable plan unit IDs (D9)

`feature.plan` gives every implementation unit a permanent ID (`U1`, `U2`, …) in
`features/<f>/plan.md`. IDs never change on reorder or split. `feature.impl` and
`feature.verify` reference units by ID, so the cursor and evidence survive
re-planning and the state machine stays resumable. This is the one CE mechanism
worth borrowing wholesale; we keep it lightweight (IDs in the plan markdown), not
a separate tracking system.

---

## Skill Shim Pattern

Each `vibe-*` skill follows the same internal sequence:

1. Read `.agents/flow/state.json` and the relevant root or feature specs.
2. On entry to a `*.design` or `*.triage` state, read `.spec/lessons.md` first (D8).
3. Confirm or transition state through `set-state.sh`.
4. Delegate to the specialized skill with the state's `reads`/`writes` paths injected.
5. Run the relevant validation or verification.
6. Summarize evidence and next legal states.

---

## Prompt Injection & Caveman (D10)

There is **one inject owner**: the adapter's `UserPromptSubmit` hook (or its
equivalent) reads the current `<flow>.<phase>` entry and emits that entry's static
`inject` string. That single inject *also* sets the caveman level — vibe
does not run a separate caveman tracker hook in parallel, because two injectors
collide and the agent follows the last one.

The inject is small (~30–60 tokens) and names: the mandatory skill, the allowed
write surface, the required output path, the caveman level, and the next legal
state. It is injected only where drift is costly (states with a mandatory skill +
forbidden writes). `idle`, `setup.detect`, and pure-read moments need no inject
beyond the legal `next` list.

Two carve-outs ride in **every** inject, regardless of caveman level — taken from
the upstream caveman skill's own rules:

1. Use normal prose for security warnings and irreversible-action confirmations;
   never compress those.
2. Caveman is **output compression only**; it never reduces reasoning depth.

Caveman level definitions are canonical in [../../product.md](../../product.md)
(Communication Levels). The inject names a level; it does not re-explain it.

---

## Prompt Cache Discipline

The injection pattern is cache-safe only if the cache boundary is respected:

- The static `state-machine.json`, tool list, and adapter prose live in the
  **cached prefix** and must stay byte-identical across turns.
- The per-turn inject rides the **user message** (post-breakpoint), which is why
  `UserPromptSubmit` is the right hook — it never mutates the cached prefix.
- Each state's `inject` is a **frozen string**. Do not template per-turn values
  (timestamps, turn counters, changed-file lists, the dropped `notes` field) into
  it, or the cache rebuilds every turn.
- `<feature>` interpolated into an inject is acceptable: it sits post-breakpoint
  and is stable within a feature session.
- Caveman compression has no cache impact — it changes output tokens, not the
  cached input prefix.
- Skill bodies load on invoke (the `Skill` tool injects a SKILL.md). Invoke lazily
  per state; do not preload the whole skill catalog.

---

## Feature-dev Subagents (D11)

We cherry-pick Anthropic's `feature-dev` subagents into specific states rather than
letting `/feature-dev` own the whole feature arc as one opaque macro. This keeps
granular state and on-disk specs while still getting the parallel exploration and
review value:

- `feature.design` → `code-explorer` (trace the codebase) + `code-architect`
  (sketch approaches). superpowers `brainstorming` drives the *human* dialogue;
  these subagents do the *codebase* work. They compose, they do not compete.
- `feature.verify` / `quick.verify` → `code-reviewer` (confidence-filtered review),
  alongside superpowers `verification-before-completion`.

feature-dev artifacts are ephemeral (in-conversation); the on-disk spec writes stay
owned by the `spec` skill and superpowers `writing-plans`.

---

## External Skill Matrix

| Code Skill | Primary External Skills | feature-dev agents | Caveman |
|---|---|---|---|
| `vibe-setup` | `spec`, `superpowers:writing-skills` | — | lite |
| `vibe-strategy` | `superpowers:brainstorming`, `spec` | — | lite |
| `vibe-feature` | `superpowers:brainstorming`, `superpowers:writing-plans`, `spec`, `superpowers:executing-plans`, `superpowers:test-driven-development`, `superpowers:subagent-driven-development` | `code-explorer`, `code-architect`, `code-reviewer` | lite (design/plan), full (impl/verify) |
| `vibe-quick` | `superpowers:systematic-debugging`, `superpowers:test-driven-development`, `superpowers:verification-before-completion` | `code-reviewer` | full |
| `vibe-verify` | `superpowers:verification-before-completion`, `superpowers:requesting-code-review`, `superpowers:systematic-debugging` | `code-reviewer` | full |
| `vibe-compound` | `spec`, `superpowers:finishing-a-development-branch` | — | lite (receipts ultra) |
| `vibe-amend` | `spec`, `superpowers:receiving-code-review` | — | lite |

These external skills are assumed installed, not bundled (only `spec` ships). A
missing delegated skill is a graceful-degradation concern tracked in the plan, not
a hard requirement of the state model.
