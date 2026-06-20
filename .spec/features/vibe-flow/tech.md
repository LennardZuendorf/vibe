---
type: feature-tech
feature: vibe-flow
sibling: product.md
parent: ../../tech.md
updated: 2026-06-18
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
**Plan:** [plan.md](plan.md)
**Related:** [../platform-adapters/tech.md](../platform-adapters/tech.md),
[../agent-instructions/tech.md](../agent-instructions/tech.md)

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
│       ├── validate-state.sh
│       └── regen-active-rules.sh
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

`state-machine.json` defines each state as a static entry. `caveman` is static per
state and the `skill` field **links** the state to its owning shim — never stored
in the mutable cursor. Under D12 the per-turn orders are sourced from that linked
skill (see *Prompt Injection*), not from a hand-written `inject` string:

```json
{
  "feature.impl": {
    "skill": "vibe-feature",
    "delegates": ["superpowers:executing-plans", "superpowers:test-driven-development"],
    "caveman": "full",
    "reads": [".spec/features/<feature>/plan.md"],
    "writes": ["src/**", "tests/**"],
    "inject": null,
    "next": ["feature.verify"],
    "exit": "tests reference plan unit IDs and pass"
  }
}
```

`set-state.sh` is the only sanctioned writer of `state.json`. Adapters and the
`/flow` command call it; they do not write the cursor directly.

### States, skills, and artifacts

Canonical reference — must match `.agents/flow/state-machine.json`. Root
[product.md](../../product.md) phase map mirrors this table.

| State | Skill | Delegates | feature-dev | caveman | R/W surface | next |
|---|---|---|---|---|---|---|
| `idle` | — | `using-superpowers` | — | lite | R `lessons.md`, `plan.md` | setup.detect, strategy.brainstorm, feature.design, quick.triage |
| `setup.detect` | `vibe-setup` | — | — | lite | R repo, adapters, `.agents/**`, `.spec/**` | setup.apply, idle |
| `setup.apply` | `vibe-setup` | `spec`, `writing-skills` | — | lite | W `.agents/**`, baseline `.spec/**`¹ | idle |
| `strategy.brainstorm` | `vibe-strategy` | `brainstorming` | — | lite | R `lessons.md`; scratch | strategy.spec |
| `strategy.spec` | `vibe-strategy` | `spec` | — | lite | W root `.spec/{product,tech,design,plan}.md` | strategy.compound, idle |
| `strategy.compound` | `vibe-compound` | `spec` | — | lite | W `lessons.md`, active-rules digest² | idle |
| `feature.design` | `vibe-feature` | `brainstorming` | `code-explorer`, `code-architect` | lite | R `lessons.md`, root specs; W `features/<f>/{product,tech}.md` | feature.plan |
| `feature.plan` | `vibe-feature` | `writing-plans` | `code-architect` | lite | W `features/<f>/plan.md` (U1…) | feature.impl |
| `feature.impl` | `vibe-feature` | `executing-plans`, `test-driven-development` | — | full | W `src/**`, `tests/**` | feature.verify |
| `feature.verify` | `vibe-verify` | `verification-before-completion`, `requesting-code-review`, `systematic-debugging` | `code-reviewer` | full | evidence only | feature.compound, feature.impl, feature.plan |
| `feature.compound` | `vibe-compound` | `finishing-a-development-branch`, `spec` | — | lite³ | W `lessons.md`, root specs, archive, active-rules² | idle |
| `quick.triage` | `vibe-quick` | `systematic-debugging` | — | full | R `lessons.md` | quick.fix, feature.design |
| `quick.fix` | `vibe-quick` | `test-driven-development` | — | full | W `src/**`; opt `.spec/quick/<slug>.md` | quick.verify |
| `quick.verify` | `vibe-verify` | `verification-before-completion` | `code-reviewer` | full | evidence only | idle |
| `amend`⁴ | `vibe-amend` | `spec`, `receiving-code-review` | — | lite | W target state's write surface only | (returns to prior state) |

¹ `setup.apply` still lists adapter active-rules in the machine **until**
[agent-instructions AI4](../agent-instructions/plan.md) narrows adapter provisioning.
² Active-rules digest via `regen-active-rules.sh`; requires symlink-aware dedupe before
compound when `CLAUDE.md` symlinks `AGENTS.md`.
³ Receipts `ultra`; body `lite`.
⁴ Modifier — never a cursor state (`set-state.sh` rejects `amend`). Invoked via
`vibe-amend` while cursor stays on the originating state; inject hook uses the
**stored cursor state**, not `amend`.

Three deliberate calls:

- **`quick.triage` is `full`, not `ultra`.** `ultra` compresses hardest and can drop
  edge cases; triage is exactly where a missed edge case is expensive. `ultra` is
  reserved for `feature.compound` receipts and subagent→orchestrator summaries.
- **`amend` is a modifier, not a flow.** The cursor never becomes `amend`
  (`set-state.sh` refuses it). `vibe-amend` applies a targeted scope edit using the
  **current cursor state's** write rules, then returns there. The inject hook always
  resolves orders from the stored cursor — not from the `amend` machine entry.
  The `amend` entry in `state-machine.json` is reference-only for `vibe-amend` skill
  prose; under D12 it does **not** receive per-turn inject.
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

Per-turn orders live in each `vibe-*` skill as a `## Orders (D12)` section of
`<!-- vibe:orders:<state> -->` … `<!-- /vibe:orders -->` blocks. Every
skill-owning state carries `inject: null` in `state-machine.json`; only `idle`
keeps an inline `inject` (skill-less fallback). `.agents/flow/scripts/orders.sh`
resolves the cursor `<flow>.<phase>` (default `idle`), follows the `skill` link,
extracts that state's block, interpolates `<feature>` (the only substitution, so
the inject stays prompt-cache stable), and prints it — degrading to the machine
inject then a generic one-liner, always exit 0. The `UserPromptSubmit` inject
hook is a thin shell over `orders.sh`. Verified by `tests/flow/run.sh`
(`vibe-flow/1`).

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
2. On entry to a `*.design` or `*.triage` state, read `.spec/lessons.md` first (D8);
   surface entries whose `**Tags:**` match the work in hand (keyword scan).
3. Confirm or transition state through `set-state.sh`.
4. Delegate to the specialized skill with the state's `reads`/`writes` paths injected.
5. Run the relevant validation or verification.
6. Summarize evidence and next legal states.

---

## Prompt Injection & Caveman (D10 → D12)

There is **one inject owner**: the adapter's `UserPromptSubmit` hook. It reads the
current `<flow>.<phase>` entry, follows that entry's `skill` link, and injects that
**skill shim's per-state orders**. The skill is the single source of the per-turn
orders, not a separate hand-written string (**D12 — supersedes the
frozen-`inject`-string mechanism of D10; D10's single-owner and cache-stable
invariants are retained**). That single inject *also* sets the caveman level —
vibe does not run a separate caveman tracker hook in parallel, because two
injectors collide and the agent follows the last one.

Why the skill is the source: the orders (mandatory skill, allowed write surface,
output path, caveman level, next legal state) were previously duplicated in both
the state machine's `inject` string and the skill body, and two copies drift apart.
Making the skill canonical removes the duplication — behaviour is edited in one
place.

Mechanics:

- `state-machine.json` keeps the `skill` field as the **link**; the hand-written
  `inject` string is dropped (`null`) for states that own a skill.
- Each `vibe-*` skill exposes a small, **machine-extractable per-`<flow>.<phase>`
  orders block** — the content the old inject carried. The hook extracts the block
  for the current state, so the inject stays small (~30–60 tokens) and cache-stable
  rather than dumping the whole shim body. (Whole-shim vs orders-block injection is
  an implementation choice flagged in the platform-adapters plan.)
- **Skill-less `idle` keeps an inline fallback.** Only `idle` retains a minimal
  inline string in `state-machine.json` after U8. `amend` is not inject-resolved
  (modifier; see States table). Skill-owning states drop `inject` to `null`.

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
- Each state's orders block (sourced from its linked skill, D12) is **static** —
  byte-identical across turns. Do not template per-turn values (timestamps, turn
  counters, changed-file lists, the dropped `notes` field) into it, or the cache
  rebuilds every turn.
- `<feature>` interpolated into the injected orders is acceptable: it sits
  post-breakpoint and is stable within a feature session.
- Caveman compression has no cache impact — it changes output tokens, not the
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

| Code Skill | Primary External Skills | feature-dev agents | Caveman |
|---|---|---|---|
| `vibe-setup` | `spec`, `superpowers:writing-skills` | — | lite |
| `vibe-strategy` | `superpowers:brainstorming`, `spec` | — | lite |
| `vibe-feature` | `superpowers:brainstorming`, `superpowers:writing-plans`, `spec`, `superpowers:executing-plans`, `superpowers:test-driven-development` | `code-explorer`, `code-architect`, `code-reviewer` | lite (design/plan), full (impl/verify) |
| `vibe-quick` | `superpowers:systematic-debugging`, `superpowers:test-driven-development`, `superpowers:verification-before-completion` | `code-reviewer` | full |
| `vibe-verify` | `superpowers:verification-before-completion`, `superpowers:requesting-code-review`, `superpowers:systematic-debugging` | `code-reviewer` | full |
| `vibe-compound` | `spec`, `superpowers:finishing-a-development-branch` | — | lite (receipts ultra) |
| `vibe-amend` | `spec`, `superpowers:receiving-code-review` | — | lite |

These external skills are assumed installed, not bundled (only `spec` ships). A
missing delegated skill is a graceful-degradation concern tracked in the plan, not
a hard requirement of the state model.
