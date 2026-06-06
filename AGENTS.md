# vibe — Codex Adapter

This file is the Codex-facing adapter for vibe. It points Codex at the
platform-neutral workflow core; it is not the source of truth. `.agents/flow` is
shared with Claude Code and any future adapter — do not invent a separate state
model.

## Canonical Core

- Durable planning: `.spec/`
- Runtime flow state: `.agents/flow/`
- Workflow skills: `.agents/skills/vibe-*`
- Spec framework skill: `.agents/skills/spec/`

## At Session Start

Sessions are ephemeral; the cursor is the memory, not the chat history.

1. Read `.agents/flow/state.json` (`{flow, phase, feature, updated}`). Missing or
   unreadable → treat as `idle`; run `validate-state.sh` if in doubt.
2. The compound `<flow>.<phase>` key indexes `.agents/flow/state-machine.json` —
   the source of truth for each state's skill, delegates, caveman level,
   read/write surface, legal `next`, and exit predicate.
3. If `idle`: read `.spec/lessons.md` and `.spec/plan.md`, then route the request
   to a flow (table below). If mid-flow: resume the current state's skill.

## Skill Routing

| Intent | Skill |
|---|---|
| Set up or repair the harness | `vibe-setup` |
| Project direction / refocus | `vibe-strategy` |
| Named feature, end to end | `vibe-feature` |
| Small bounded fix | `vibe-quick` |
| Evidence before "done" | `vibe-verify` |
| Lessons, promotion, archive | `vibe-compound` |
| Mid-flight scope correction | `vibe-amend` |

`vibe-*` skills are shims: they delegate to `spec`, `superpowers:*`, and subagents,
always naming the exact `.spec/` paths that may be written.

## Lifecycle: start → continuous

vibe is a **spiral, not a pipeline** — each loop should make the next cheaper.

1. **Bootstrap once.** `vibe-setup`: audit then merge the harness without
   clobbering. → `idle`.
2. **Set direction.** `vibe-strategy`: brainstorm, then write root
   `.spec/{product,tech,design,plan}.md`. → `idle`.
3. **Build, repeatedly.** `vibe-feature`: design → plan (stable unit IDs `U1…`,
   **human gate**) → impl (test-first, cite the IDs) → verify (**human gate**) →
   compound. `vibe-quick` is the low-ceremony path for small fixes.
4. **Compound — do not skip it.** `vibe-compound` records a tagged lesson, promotes
   cross-cutting decisions into root specs, archives the feature, and regenerates
   the active-rules digest below. The return arrow — lessons read back at the next
   `design`/`triage` — is the whole point.

Plan and verify are meant to outweigh raw implementation time; the human gates
force that. `verify` checks observable behaviour against the plan's units.

## Flow State & Transitions

- **Transition only via** `bash .agents/flow/scripts/set-state.sh <flow.phase>
  [feature]`. Never edit `state.json` directly. Check the current state's `next`
  before moving; transitions are suggested, then confirmed.
- **Per-turn orders come from the linked skill (D12).** The state's `skill` field
  links to its shim; the orders (skill, write surface, output path, caveman level,
  next) are sourced from that skill, not a hand-written string. Codex has no inject
  hook, so consult the linked skill's current-state orders each turn and follow
  them. Skill-less states (`idle`, `amend`) keep a minimal inline fallback.
- **Recovery.** `validate-state.sh` (cursor sanity), `detect-context.sh` (snapshot
  + allow/warn/block write decision — the one place the write policy lives),
  `regen-active-rules.sh` (rebuild the digest during compound). A stale cursor can
  mislead even when `.spec/` is correct; re-validate when in doubt.

## Write Invariants (the three hard blocks)

The decision policy lives once, in `detect-context.sh decide`. Hard blocks:

1. `.spec/lessons.md` — writable only in a `*.compound` state.
2. Root `.spec/{product,tech,design,plan}.md` — only in `strategy.spec` or
   `feature.compound` (plus `setup.apply` bootstrap).
3. `.agents/flow/state.json` — only via `set-state.sh`.

Everything else is allow/warn.

## Caveman Density

Output compression only — never reasoning. Code, paths, and commands stay
byte-exact; security warnings and irreversible-action confirmations stay in normal
prose at every level (definitions canonical in `.spec/product.md`):

- `lite` — setup, strategy, design, plan, compound, amend.
- `full` — impl, verify, quick.* (the working default).
- `ultra` — compound receipts and subagent→orchestrator summaries only; never triage.

## Spec Layout

Root: `.spec/{product,tech,design,plan,lessons}.md`. Features:
`.spec/features/<feature>/{product,tech}.md` required, with optional `design.md`,
`plan.md`, `research.md`. Validate with `bash .agents/skills/spec/scripts/validate.sh`.

The active-rules block below is **generated** from `.spec/lessons.md` by
`regen-active-rules.sh`; edit `lessons.md` during compound, not the block.

<!-- vibe:active-rules:start -->
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->
