---
type: concept
updated: 2026-10-19
---

# Vibe Flow Contract

Cursor JSON is unchanged in shape, at a new path,
`.vibe/run/flow/state.json`:

```json
{
  "flow": "idle | setup | strategy | feature | quick",
  "phase": "idle | detect | apply | brainstorm | spec | design | plan | impl | verify | compound | triage | fix",
  "feature": null,
  "updated": "2026-06-02T00:00:00Z"
}
```

`state-machine.json` defines each `<flow>.<phase>` state's phase-file link,
delegates, write surface, exit predicate, and legal `next`; edge-keyed
`gates` name the two human approvals. **The writer is the gate:** `vibe-flow
state set` refuses a target outside `next` and a gated edge without
`--confirm` — `/flow` becomes a thin wrapper over the same check. A
`.vibe/run/flow/ledger.jsonl` records every transition, hook verdict, and
advisory signal; `vibe-flow tui` reads the same cursor and ledger for a live
pane and recorded-flow playback. Full contract: [tech-rust.md](../../../tech/rust.md).
