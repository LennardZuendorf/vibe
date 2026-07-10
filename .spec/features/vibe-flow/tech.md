---
type: feature-tech
feature: vibe-flow
sibling: product.md
parent: ../../tech.md
updated: 2026-07-10
---

# Feature: Vibe Flow — Architecture

The vibe flow is a platform-neutral runtime layer under `.agents/skills/vibe`: one
`vibe` agent skill — a router `SKILL.md` plus per-phase files — driven by a static
state machine. Its job is to carry the planning load: the user says "I need X" and
the flow constrains the agent into the right phase, with the right delegates, the
right output path, and one machine-level output `style` already chosen for it.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)
**Design:** [design.md](design.md)
**Plan:** [plan.md](plan.md)
**Related:** [../platform-adapters/tech.md](../platform-adapters/tech.md),
[../agent-instructions/tech.md](../agent-instructions/tech.md)

---

## Files

```text
.agents/
└── skills/
    └── vibe/
        ├── SKILL.md            # lean router + the 12 skill-owning states' orders blocks
        ├── setup.md            # phase files: one per flow group
        ├── strategy.md
        ├── feature.md
        ├── quick.md
        ├── verify.md
        ├── compound.md
        ├── state-machine.json  # static flow definition
        ├── state.example.json  # neutral cursor template
        ├── scripts/            # set-state.sh, detect-context.sh, orders.sh, …
        └── reference/          # adapters.json, templates/
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

`state-machine.json` defines each state as a static entry; the `skill` field
**links** the state to the `vibe` skill — never stored in the mutable cursor.
Under D12 the per-turn orders are sourced from that linked skill (see *Prompt
Injection*), not from a hand-written `inject` string. There is no per-state
`caveman` field: output density is one top-level `style` note for the whole
machine (see *States, skills, and artifacts*):

```json
{
  "feature.impl": {
    "skill": "vibe",
    "delegates": ["superpowers:executing-plans", "superpowers:subagent-driven-development", "superpowers:test-driven-development", "superpowers:receiving-code-review"],
    "reads": [".spec/features/<feature>/plan.md"],
    "writes": ["src/**", "tests/**"],
    "inject": null,
    "next": ["feature.verify", "idle"],
    "exit": "plan units implemented with tests that cite their unit IDs and pass"
  }
}
```

`set-state.sh` is the only sanctioned writer of `state.json`. Adapters and the
`/flow` command call it; they do not write the cursor directly.

### States, skills, and artifacts

**Canonical: [`flow/state-machine.json`](../../../flow/state-machine.json)**
(runtime path `.agents/skills/vibe/state-machine.json`); the root
[product.md](../../product.md) phase map is the human-readable mirror. A full
per-state table used to live here, but it was **retired 2026-07-09** — a
hand-maintained copy of the machine drifts, and this one had gone stale
(pre-consolidation `vibe-setup`/`vibe-strategy` skill names, no per-phase
compound states, no edge-keyed `gates`). Read the machine for the authoritative
per-state skill link, delegates, write surface, and legal `next`; output density
is one top-level `style` note (not a per-state level), and the flow carries a
top-level `gates` object keyed by edge
(`feature.plan>feature.impl`, `feature.verify>feature.compound`).

The machine now carries **13 state entries** (`idle` plus 12 non-idle states), one
top-level `style` note, and no per-state modifiers. Three deliberate calls in it
are worth spelling out — but `flow/state-machine.json` is the per-state truth:

- **Output density is one machine-level `style` note, not a per-state level.** The
  per-state caveman levels (`lite`/`full`/`ultra`) and the `caveman_levels` /
  `safety_carveouts` machine blocks were retired 2026-07-09; a single top-level
  `style` now governs every state — compress receipts and subagent→orchestrator
  summaries, but keep security warnings and irreversible-action confirmations in
  full prose, and never trade reasoning depth for brevity.
- **A scope edit is not a state.** There is no `amend` flow or modifier — the cursor
  never becomes `amend`, and the `modifiers` array is gone (removed with `amend.md`).
  A targeted scope edit happens within the current cursor state's write surface and
  the cursor stays put; `set-state.sh idle` still aborts any flow.
- **Failure routes back.** `feature.verify` can return to `feature.plan` or
  `feature.impl`, which resolves the failed-verification routing the plan left open.

### Transition policy

- **`set-state.sh` is the writer, not the gate.** It validates cursor shape and
  writes atomically; it does **not** check whether the target ∈ current state's
  `next`. Illegal transitions are refused by the **agent** (and eventually the
  `/flow` command) before calling the script.
- **`validate-state.sh`** checks cursor sanity (shape, machine key exists); not
  transition history.
- Future optional: legality check flag on `set-state.sh` — not implemented in Stage 1.

### D12 implementation status (delivered)

Per-turn orders live in the `vibe` skill's `SKILL.md` as a `## Orders (D12)` section of
`<!-- vibe:orders:<state> -->` … `<!-- /vibe:orders -->` blocks — one block per
skill-owning state (12 today). Every
skill-owning state carries `inject: null` in `state-machine.json`; only `idle`
keeps an inline `inject` (skill-less fallback). `.agents/skills/vibe/scripts/orders.sh`
resolves the cursor `<flow>.<phase>` (default `idle`), follows the `skill` link,
extracts that state's block, interpolates `<feature>` (the only substitution, so
the inject stays prompt-cache stable), and prints it — degrading to the machine
inject then a generic one-liner, always exit 0. The `UserPromptSubmit` inject
hook is a thin shell over `orders.sh`. Verified by `flow/tests/run.sh`
(`vibe-flow/1`).

### Stable plan unit IDs (D9)

`feature.plan` gives every implementation unit a permanent ID (`<feature>/1`,
`<feature>/2`, …) in `features/<f>/plan.md`. IDs never change on reorder or split. `feature.impl` and
`feature.verify` reference units by ID, so the cursor and evidence survive
re-planning and the state machine stays resumable. This is the one CE mechanism
worth borrowing wholesale; we keep it lightweight (IDs in the plan markdown), not
a separate tracking system.

---

## Phase file pattern

The `vibe` skill's `SKILL.md` router hands each state to its phase file
(`setup.md`, `strategy.md`, `feature.md`, `quick.md`, `verify.md`, `compound.md`).
Every phase follows the same internal sequence:

1. Read `.agents/skills/vibe/state.json` and the relevant root or feature specs.
2. On entry to a `*.design` or `*.triage` state, read `.spec/lessons.md` first (D8);
   surface entries whose `**Tags:**` match the work in hand (keyword scan).
3. Confirm or transition state through `set-state.sh`.
4. Delegate to the specialized skill with the state's `reads`/`writes` paths injected.
5. Run the relevant validation or verification.
6. Summarize evidence and next legal states.

---

## Prompt Injection (D10 → D12)

There is **one inject owner**: the adapter's `UserPromptSubmit` hook. It reads the
current `<flow>.<phase>` entry, follows that entry's `skill` link, and injects that
state's **orders block** from the `vibe` skill's `SKILL.md`. The skill is the single
source of the per-turn orders, not a separate hand-written string (**D12 —
supersedes the frozen-`inject`-string mechanism of D10; D10's single-owner and
cache-stable invariants are retained**).

Why the skill is the source: the orders (mandatory delegates, allowed write surface,
output path, next legal state) were previously duplicated in both the state machine's
`inject` string and the skill body, and two copies drift apart. Making the skill
canonical removes the duplication — behaviour is edited in one place.

Mechanics:

- `state-machine.json` keeps the `skill` field as the **link**; the hand-written
  `inject` string is dropped (`null`) for states that own a skill.
- The `vibe` skill exposes a small, **machine-extractable per-`<flow>.<phase>`
  orders block** — the content the old inject carried. The hook extracts the block
  for the current state, so the inject stays small (~30–60 tokens) and cache-stable
  rather than dumping the whole phase body.
- **Skill-less `idle` keeps an inline fallback.** Only `idle` retains a minimal
  inline string in `state-machine.json`; every skill-owning state drops `inject` to
  `null`. (There is no `amend` state — a scope edit stays within the current state.)

Output density is **not** carried per-state. A single top-level `style` note in
`state-machine.json` governs every state: no filler; compress receipts and subagent
summaries; keep security warnings and irreversible-action confirmations in full
prose; never trade reasoning depth for brevity. The per-state `caveman` levels
(`lite`/`full`/`ultra`) this section once described were **retired 2026-07-09** in
favour of that one note.

---

## Prompt Cache Discipline

The injection pattern is cache-safe only if the cache boundary is respected:

- The static `state-machine.json`, tool list, and adapter prose live in the
  **cached prefix** and must stay byte-identical across turns.
- The per-turn inject rides the **user message** (post-breakpoint), which is why
  `UserPromptSubmit` is the right hook — it never mutates the cached prefix.
- Each state's orders block (sourced from its linked skill, D12) is **static** —
  byte-identical across turns. Do not template per-turn values (timestamps, turn
  counters, changed-file lists, the dropped `notes` field) into it, or the cache
  rebuilds every turn.
- `<feature>` interpolated into the injected orders is acceptable: it sits
  post-breakpoint and is stable within a feature session.
- `style`-note compression has no cache impact — it changes output tokens, not the
  cached input prefix.
- The hook injects only the current state's **orders block**, not the whole skill
  body. Full SKILL.md bodies still load lazily on `Skill`-tool invoke; do not
  preload the whole skill catalog.

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

Delegates are keyed per state, not per skill — the one `vibe` skill routes each
`<flow>.<phase>` to the executors below. **`state-machine.json` `delegates` is the
authoritative list;** this table mirrors it for reading. There is no per-state
output level — density is the single top-level `style` note.

| State | Primary external skills | feature-dev subagents |
|---|---|---|
| `setup.detect` | — | — |
| `setup.apply` | `spec` | — |
| `strategy.brainstorm` | `superpowers:brainstorming` | — |
| `strategy.spec` | `spec` | — |
| `feature.design` | `superpowers:brainstorming` | `code-explorer`, `code-architect` |
| `feature.plan` | `superpowers:writing-plans` | `code-architect` |
| `feature.impl` | `superpowers:executing-plans`, `superpowers:subagent-driven-development`, `superpowers:test-driven-development`, `superpowers:receiving-code-review` | — |
| `feature.verify` | `superpowers:verification-before-completion`, `superpowers:requesting-code-review`, `superpowers:systematic-debugging` | `code-reviewer` |
| `feature.compound` | `superpowers:finishing-a-development-branch`, `spec` | — |
| `quick.triage` | `superpowers:systematic-debugging` | — |
| `quick.fix` | `superpowers:test-driven-development`, `superpowers:receiving-code-review` | — |
| `quick.verify` | `superpowers:verification-before-completion` | `code-reviewer` |

These external skills are assumed installed, not bundled (only `spec` ships). A
missing delegated skill is a graceful-degradation concern tracked in the plan, not
a hard requirement of the state model.
