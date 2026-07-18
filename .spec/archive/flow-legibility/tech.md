---
type: feature-tech
feature: flow-legibility
sibling: product.md
parent: ../../tech.md
updated: 2026-07-18
---

# Feature: flow-legibility — Architecture

Five focused edits to the existing flow engine and Claude adapter — no new
subsystem. Each closes one legibility gap while preserving the warn-first,
graceful-degrade, byte-stable-inject invariants the current tests pin. The
`SessionStart` hook and `doctrine.sh` resolver are the only new files; everything
else is an in-place change to a script, the machine, the orders blocks, the
delegation contracts, or a test suite.

**Parent:** [../../tech.md](../../tech.md)
**Requirements:** [product.md](product.md)

---

## Files

```
flow/SKILL.md                       # orders blocks → imperative; +<!-- vibe:doctrine --> block; +model-tier note
flow/state-machine.json             # idle inject → imperative; +2 next edges; research.md in feature.design writes
flow/scripts/doctrine.sh            # NEW — resolver: extract doctrine block + cursor summary (mirrors orders.sh)
flow/scripts/detect-context.sh      # +infer_state(): activity→likely-state (warn-only, no new blocks)
flow/scripts/doctor.sh              # +instruction-coverage check (doctrine block present OR SessionStart wired)
flow/scripts/merge-settings.sh      # +SessionStart block builder/merger (idempotent)
flow/feature.md,strategy.md,quick.md,verify.md,compound.md  # +model tier per Delegate block
flow/reference/templates/AGENTS.md  # session-start doctrine sections sourced from the shared block
.claude/hooks/session-start-doctrine.sh   # NEW — thin shell over doctrine.sh
.claude/hooks/user-prompt-submit-inject.sh # prepend drift nudge as line 1 when detect-context infers drift
.claude/settings.json               # +SessionStart wiring (installer-written via merge-settings.sh)
flow/tests/run.sh                   # +imperative-orders, loop-edge, tier, byte-budget assertions; update strategy.spec.next test
flow/tests/adapters/run.sh          # +SessionStart wiring, doctrine output, drift-first order, doctor coverage
```

---

## Contract / API

**`doctrine.sh`** — same self-location + jq-optional pattern as `orders.sh`.
`bash doctrine.sh` → stdout = the `<!-- vibe:doctrine -->` block from
`flow/SKILL.md` (markers stripped) followed by a one-line cursor summary derived
from `state.json` (`flow.phase` + feature, or `idle`). Base text is byte-stable;
only the cursor line varies. Exit 0 with no output if the block/dir is missing.

**`session-start-doctrine.sh`** — `ROOT="${CLAUDE_PROJECT_DIR:-$PWD}"`; runs
`doctrine.sh`; prints its stdout (SessionStart `additionalContext`); graceful
exit 0. No arguments; source-agnostic (the matcher decides when it fires).

**`settings.json` SessionStart** — `merge-settings.sh` adds a `SessionStart`
array whose matcher covers first-load and compaction sources
(`startup|resume|compact`; `compact` = re-inject), command
`bash "$CLAUDE_PROJECT_DIR/.claude/hooks/session-start-doctrine.sh"`, timeout 10.
Built by the same merge idiom as the existing three events, so re-runs are no-ops
and user keys survive.

**`detect-context.sh infer`** — new subcommand, read-only. Reads `state.json`
and `git status --porcelain` (or a passed-in porcelain string for testability),
returns `drift:<suggested-state>:<reason>` on stdout when activity contradicts
the cursor (rule table below), else nothing. `decide <path>` is unchanged — the
three hard blocks and warn classes stay exactly as they are.

**Inject prepend** — `user-prompt-submit-inject.sh` calls `detect-context.sh
infer`; on a `drift:` result it prints `vibe-drift: <reason> → run /flow
<state>` as the first line, then the orders block, then the drained
`warnings.log`. No drift → unchanged ordering (orders block first, byte-stable).

**Orders imperative form** — each `vibe:orders:<state>` block's `· next: X` tail
becomes `· → set-state.sh X` (non-gated) or `· HUMAN GATE → /flow X confirm`
(gated source), keeping the `gate:` marker where required and staying ≤400 bytes.
The `idle` inline inject in `state-machine.json` gets the same imperative form.

**Model-tier line** — each `> **Delegate — X**` block gains a `> - model: sonnet`
or `> - model: opus` line; `flow/SKILL.md` carries a one-line policy statement.
Tier map: `code-explorer` → sonnet; `code-architect`, `code-reviewer`, and any
review/architecture/synthesis Task dispatch → opus.

---

## Implementation Detail

### Drift inference rule table (conservative, warn-only)

| Cursor | Activity | `infer` output |
|---|---|---|
| `idle` | uncommitted `src/**` or `tests/**` | `drift:feature.impl:src edits at idle → feature.impl or quick.fix` |
| `feature.design`/`feature.plan` | uncommitted `src/**` | `drift:feature.impl:building before the impl gate` |
| any impl/fix/verify state | — | none (activity is consistent) |

Only clear contradictions emit; ambiguity stays silent. The nudge names a
one-command fix and never blocks, so a false positive costs one advisory line.

### Loop edges + machine schema

Add `strategy.brainstorm` to `strategy.spec.next` and `feature.design` to
`feature.plan.next`. `research.md` joins `feature.design`'s `writes` array; no
`research` phase is added (`phases` array unchanged). The gates↔next test,
idle-abort sweep, and every-`next`-target-known test continue to pass; the
`strategy.spec.next is exactly [idle]` assertion in `flow/tests/run.sh` is
updated to `[strategy.brainstorm, idle]`.

<!-- merge -->
## Injection-first legibility (promotes to root tech.md at compound)

The flow's comprehension and self-drive move from the `AGENTS.md` managed block
into the injection layer: (1) per-turn orders are **imperative and
self-carrying** — every inject names the `set-state.sh <next>` transition (or the
`/flow <next> confirm` gate), so the duty no longer depends on re-reading
phase-file prose; (2) a `SessionStart` doctrine hook (+`compact` re-inject),
sourced from a single `<!-- vibe:doctrine -->` block shared with the `AGENTS.md`
template, delivers the working model each session, making the `AGENTS.md` block
an optional adapter; (3) cursor-drift is inferred from working-tree activity and
surfaced as the first inject line. This flips the two `Doctrine`/`Redirect`
"planned" hook-table rows: `SessionStart` doctrine becomes live here (the
`PostToolUse` redirect stays with delegation-redirect). All of it stays
warn-first and jq-optional — legibility and agency, not new hard rails.
<!-- /merge -->

### Single-source doctrine (D12 parallel)

`doctrine.sh` extracts the block exactly as `orders.sh` extracts
`vibe:orders:*` — same marker-sed, same self-location, same jq-optional cursor
read — so the hook and the `AGENTS.md` template never carry divergent copies
(the 2026-06-18 single-source lesson). The `AGENTS.md` template's Session-start /
Working-model / Write-invariants sections are the human-readable rendering of the
same block.

---

## Open Questions

1. **`SessionStart` `additionalContext` + matcher semantics** — verify at build
   that a `SessionStart` hook's stdout reaches the model as context and that
   `compact` is a supported source for re-inject. Degrade path if not: keep the
   `AGENTS.md` block as the doctrine carrier and wire `SessionStart` for the
   supported sources only.
2. **Byte-budget headroom** — imperative tails + any inline tier tokens must keep
   every orders block ≤400 bytes; if a block is tight, the tier lives only in the
   phase-file Delegate block (not the compressed orders line). The existing
   budget test is the guard.
