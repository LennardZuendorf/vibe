---
type: product-feedback
scope: product
covers: fable-5 product review 2026-07-18, threaded to rework features
updated: 2026-07-18
---

# Product Feedback — 2026-07-18

A Fable 5 product-engineer review of the shipped flow-legibility feature surfaced these; each is threaded to a rework feature or marked a new candidate, and this file should be pruned as rows absorb it.

| Item | Severity | Thread to |
|---|---|---|
| Drift nudges inert on most repos | P0 | delegation-redirect (11) / config change |
| "AGENTS.md optional" is a promissory note | P0 | vibe-plugin (13) |
| No worked first-run in README | P0 | doc pass (in progress) |
| Model-tier pins are prose thrice-removed from enforcement | P1 | delegation-redirect (11) / machine data |
| Cursor fragility | P1 | new: cursor-lifecycle |
| Internal IDs (D12, R-numbers) leak into user docs | P1 | doc pass (partly fixed) |
| Is a 13-state machine right-sized for one person? | P1 (strategic) | owner decision |
| Session-scoped pause, doctrine-prose consolidation, opus cost knob | P2 | owner decision / backlog |

- **Drift nudges inert on most repos** — `detect-context.sh infer` only matches `src/`\|`tests/`; vibe's own code (`flow/`, `spec/`) and Go/monorepo/non-JS repos never fire the nudge, so R4 is demo-shaped. Fix: data-driven code globs (`code_globs` key in `state-machine.json` or `.spec/.config.yaml`).
- **"AGENTS.md optional" is a promissory note** — the SessionStart hook only ships via committed `.claude/settings.json`; uncontrolled-team delivery (plugin / `--local`) is deferred, so the claim doesn't hold today. Docs being scoped now.
- **No worked first-run in README** — being fixed now in a doc pass.
- **Model-tier pins are prose thrice-removed from enforcement** — hardcoded sonnet/opus strings across 5 phase files will rot. Fix: one data map (machine or `deps.json`) with abstract names (fast/deep).
- **Cursor fragility** — gitignored `state.json` + one-agent assumption: worktrees start `idle` while main says `feature.impl`, stale cursors get reported confidently, concurrent sessions race. Fix: doctor/doctrine staleness note + documented worktree behavior.
- **Internal IDs leak into user docs** — D12, R-numbers surface in prose meant for end users. Partly fixed in the doc pass.
- **Is a 13-state machine right-sized for a one-person workflow?** — strategic; the brainstorm's "collapse to three arcs" experiment (e.g. merge `quick.triage`+`quick.fix`) is unrun.
- **P2 grab-bag** — session-scoped pause (`vibe off` / env var the hooks respect); consolidate working-model prose (doctrine block + AGENTS template + two READMEs); a cost knob for opus-pinned dispatches.

Full verbatim review lives in this session's transcript (Fable 5 product review, 2026-07-18).
