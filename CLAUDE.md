# vibe — Claude Code Adapter

This file is the Claude Code-facing adapter for vibe. It is not the workflow
source of truth — it points Claude at the platform-neutral core and says how to
behave each session.

## Canonical Core

- Durable planning lives in `.spec/`.
- Runtime flow state lives in `.agents/flow/`.
- Workflow shims are agent skills under `.agents/skills/vibe-*`.
- Platform files under `.claude/` read the core; they do not own a separate state
  model. Do not treat `.claude/state.json`, `.claude/state-machine.json`, or
  `.claude/skills/` as canonical.

## At Session Start

Sessions are ephemeral; the cursor is the memory, not the chat history.

1. Read `.agents/flow/state.json` (`{flow, phase, feature, updated}`). Missing or
   unreadable → treat as `idle`; run `validate-state.sh` if in doubt.
2. The compound `<flow>.<phase>` key indexes `.agents/flow/state-machine.json` —
   the source of truth for each state's skill, delegates, caveman level,
   read/write surface, legal `next`, and exit predicate.
3. If `idle`: read `.spec/lessons.md` and `.spec/plan.md`, then route the request
   to a flow (table below).
4. If mid-flow: resume the current state's skill; don't jump phases. Follow the
   injected orders literally.

## Workflow Rule (routing)

When the user asks for strategy, feature work, a quick fix, verification, or
compounding, prefer the matching `vibe-*` skill:

| Intent | Skill |
|---|---|
| Set up or repair the harness | `vibe-setup` |
| Project direction / refocus | `vibe-strategy` |
| Named feature, end to end | `vibe-feature` |
| Small bounded fix | `vibe-quick` |
| Evidence before "done" | `vibe-verify` |
| Lessons, promotion, archive | `vibe-compound` |
| Mid-flight scope correction | `vibe-amend` |

Each `vibe-*` skill injects exact `.spec/` output paths when it delegates to
`spec`, `superpowers:*`, or subagents.

## Lifecycle: start → continuous

vibe is a **spiral, not a pipeline** — each loop should make the next cheaper.

1. **Bootstrap once.** `vibe-setup`: audit (`setup.detect`), then merge the harness
   (`setup.apply`) without clobbering existing content. → `idle`.
2. **Set direction.** `vibe-strategy`: brainstorm, then write root
   `.spec/{product,tech,design,plan}.md`. → `idle`.
3. **Build, repeatedly.** `vibe-feature`: design → plan (stable unit IDs `U1…`,
   **human gate**) → impl (test-first, cite the IDs) → verify (**human gate**) →
   compound. `vibe-quick` is the low-ceremony path for small bounded fixes
   (triage → fix → verify).
4. **Compound — do not skip it.** `vibe-compound` records a tagged lesson, promotes
   cross-cutting decisions into root specs, archives the feature, and regenerates
   the active-rules digest below. The return arrow — lessons read back at the next
   `design`/`triage` — is the whole point; skip it and vibe degrades to a plain
   spec tool.

Plan and verify are meant to outweigh raw implementation time; the human gates
exist to force that. `verify` checks *observable behaviour against the plan's
units*, not the agent's own assertions.

## Flow State & Transitions

- **Transition only via** `bash .agents/flow/scripts/set-state.sh <flow.phase>
  [feature]`. Never edit `state.json` directly. Check the current state's `next`
  before moving; refuse illegal transitions. Transitions are agent-*suggested* —
  name the next state and confirm before calling the script.
- **Per-turn orders come from the linked skill (D12).** The state's `skill` field
  links to its shim; the orders (skill, write surface, output path, caveman level,
  next) are sourced from that skill, not a hand-written string. Once the flow
  hooks ship, the `UserPromptSubmit` inject delivers them every turn; until then,
  read the linked skill's current-state orders yourself. Follow them literally.
  Skill-less states (`idle`, `amend`) keep a minimal inline fallback.
- **Recovery.** `validate-state.sh` (cursor sanity), `detect-context.sh` (state
  snapshot + allow/warn/block write decision — the one place the write policy
  lives), `regen-active-rules.sh` (rebuild the digest during compound). A stale
  cursor can mislead even when `.spec/` is correct; re-validate when in doubt.

## Write Invariants (the three hard blocks)

The decision policy lives once, in `detect-context.sh decide`. Hard blocks:

1. `.spec/lessons.md` — writable only in a `*.compound` state.
2. Root `.spec/{product,tech,design,plan}.md` — only in `strategy.spec` or
   `feature.compound` (plus `setup.apply` bootstrap).
3. `.agents/flow/state.json` — only via `set-state.sh`.

Everything else is allow/warn.

## Caveman Density

Output **compression only** — never reasoning depth; code, paths, and commands
stay byte-exact; security warnings and irreversible-action confirmations stay in
normal prose at every level. Levels are canonical in `.spec/product.md`:

- `lite` — setup, strategy, design, plan, compound, amend.
- `full` — impl, verify, quick.* (the working default).
- `ultra` — compound receipts and subagent→orchestrator summaries only; never triage.

One inject owner sets the level; do not run a separate caveman tracker in parallel.

## Flow State & Transitions

- **Read state:** `.agents/flow/state.json` = `{flow, phase, feature, updated}`.
  The compound `<flow>.<phase>` key indexes `.agents/flow/state-machine.json`,
  which holds each state's skill, delegates, caveman level, read/write surface,
  frozen `inject` string, and legal `next` array.
- **Transition only via** `bash .agents/flow/scripts/set-state.sh <flow.phase>
  [feature]`. Never edit `state.json` directly. Check the current state's `next`
  before moving; refuse illegal transitions. Transitions are agent-*suggested* —
  name the next state and confirm before calling the script.
- **Follow the inject literally.** It names the one skill, the write surface, the
  output path, the caveman level, and the next legal state.
- **Helper scripts:** `validate-state.sh` (cursor sanity), `detect-context.sh`
  (state snapshot + allow/warn/block write decision), `regen-active-rules.sh`
  (rebuild the active-rules digest below from `lessons.md` during compound).

The active-rules block at the bottom of this file is **generated** from
`.spec/lessons.md` by `regen-active-rules.sh`. Do not edit inside its markers;
edit `lessons.md` during a compound phase and re-run the script.

## Spec Framework

Use `.agents/skills/spec/SKILL.md` for spec navigation and validation. Root model:
`product.md`, `tech.md`, `design.md`, `plan.md`, `lessons.md`. Feature specs live
under `.spec/features/<feature>/` (`product.md` + `tech.md` required; `design.md`,
`plan.md`, `research.md` optional). Validate with
`bash .agents/skills/spec/scripts/validate.sh`.

The active-rules block below is **generated** from `.spec/lessons.md` by
`regen-active-rules.sh`. Do not edit inside its markers; edit `lessons.md` during a
compound phase and re-run the script.

<!-- vibe:active-rules:start -->
<!-- Generated from .spec/lessons.md by regen-active-rules.sh. Do not edit by hand;
     edit lessons.md and re-run during compound. Top 5, pinned first. -->

### Active Rules

_No lessons recorded yet._
<!-- vibe:active-rules:end -->
